//! `/item`: slot resolution and the shared read/modify/write execution.

use super::blocks::loaded_block_pos;
use super::give::item_stack_argument;
use super::{
    damage_entity_targets, integer_argument, single_damage_entity_argument, CommandDamageEntityRef,
    CommandRun, Outcome,
};
use crate::commands::item_modifier::{native_item_argument_stack, CommandItemModifier};
use crate::commands::parse::ExecuteItemSource;
use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::CommandOrigin;
use crate::commands::CommandPlayerTarget;
use crate::commands::{translatable, CommandCx, CommandSource};
use crate::world::ServerWorld;
use crate::world::WorldItemAccess;
use bbb_protocol::{packets::BlockPos, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::save::Java26LevelKey;
use bbb_server_types::slot_ranges::{equipment_slot_for_command_slot, player_slot, PlayerSlot};

type Result<T> = std::result::Result<T, NbtSummaryValue>;

pub(super) fn count_items(
    input: &ExecuteItemSource,
    slots: &[i32],
    predicate: &crate::commands::item_predicate::CommandItemPredicate,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Result<i32> {
    let inputs = match input {
        ExecuteItemSource::Block(pos) => {
            let pos =
                super::blocks::loaded_block_pos_of(*pos, world.command_blocks(&at.level), at)?;
            slots
                .iter()
                .map(|&slot| ItemSlot::Block { pos, slot })
                .collect::<Vec<_>>()
        }
        ExecuteItemSource::Entity(selector) => world.with_entity_reads(|reads| {
            Ok::<_, NbtSummaryValue>(
                super::required_command_entities(selector, source, at, reads)?
                    .into_iter()
                    .flat_map(|entity| {
                        slots
                            .iter()
                            .map(move |&slot| entity_slot(entity, slot, reads))
                    })
                    .collect::<Vec<_>>(),
            )
        })?,
    };
    let resolver = world.item_stack_resolver();
    let mut count = 0_i32;
    for input in inputs {
        let Some(stack) = world.read_command_item_slot(&at.level, &input, None)? else {
            continue;
        };
        let tool = crate::loot_runtime::LootTool {
            item_id: stack.item_id(),
            count: stack.count(),
            stack,
            enchantments: Default::default(),
        };
        if predicate
            .matches(&tool, &resolver)
            .map_err(|error| bbb_protocol::component_literal_nbt(error.to_string()))?
        {
            count = count.saturating_add(tool.count);
        }
    }
    Ok(count)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ItemSlot {
    Block {
        pos: BlockPos,
        slot: i32,
    },
    Player {
        player: CommandPlayerTarget,
        slot: i32,
    },
    Stored {
        uuid: uuid::Uuid,
        level: Java26LevelKey,
        name: NbtSummaryValue,
        slot: i32,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum ItemInput {
    Stack(RuntimeItemStack),
    Slot(ItemSlot),
    Modify,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemCommand {
    pub(crate) level: Java26LevelKey,
    pub(crate) origin: [f64; 3],
    pub(crate) source: CommandSource,
    input: ItemInput,
    targets: Vec<ItemSlot>,
    modifier: Option<CommandItemModifier>,
}

impl ItemCommand {
    // ItemCommands.java:311-357,380-406: each reached write precedes success and callback.
    pub(crate) fn execute(
        &self,
        access: &mut WorldItemAccess<'_>,
    ) -> Result<(i32, NbtSummaryValue)> {
        let modifying = matches!(self.input, ItemInput::Modify);
        let input = match &self.input {
            ItemInput::Stack(stack) => Some(stack.clone()),
            ItemInput::Slot(slot) => {
                let stack = access
                    .read(slot, false)?
                    .ok_or_else(|| no_slot(slot, false))?;
                Some(self.modified(access, copied_stack(stack))?)
            }
            ItemInput::Modify => None,
        };
        let mut changed = Vec::new();
        for target in &self.targets {
            let stack = if modifying {
                let Some(stack) = access.read(target, true)? else {
                    continue;
                };
                self.modified(
                    access,
                    if matches!(target, ItemSlot::Block { .. }) {
                        stack
                    } else {
                        copied_stack(stack)
                    },
                )?
            } else {
                input.as_ref().expect("replace has an input").clone()
            };
            if access.write(
                target,
                if matches!(target, ItemSlot::Block { .. }) {
                    stack.clone()
                } else {
                    copied_stack(stack.clone())
                },
            )? {
                changed.push((target, stack));
            }
        }
        let Some((first, stack)) = changed.first() else {
            let slot = self.targets.first().map_or(0, slot_number);
            return Err(if modifying {
                translatable(
                    "commands.item.target.no_changes",
                    vec![NbtSummaryValue::Int(slot)],
                )
            } else {
                translatable(
                    "commands.item.target.no_changed.known_item",
                    vec![
                        access.display(input.as_ref().expect("replace input"))?,
                        NbtSummaryValue::Int(slot),
                    ],
                )
            });
        };
        let count = i32::try_from(changed.len()).unwrap_or(i32::MAX);
        let feedback = match first {
            ItemSlot::Block { pos, .. } => translatable(
                "commands.item.block.set.success",
                vec![
                    NbtSummaryValue::Int(pos.x),
                    NbtSummaryValue::Int(pos.y),
                    NbtSummaryValue::Int(pos.z),
                    access.display(stack)?,
                ],
            ),
            ItemSlot::Player { player, .. } if count == 1 => translatable(
                "commands.item.entity.set.success.single",
                vec![access.player_name(player), access.display(stack)?],
            ),
            ItemSlot::Stored { name, .. } if count == 1 => translatable(
                "commands.item.entity.set.success.single",
                vec![name.clone(), access.display(stack)?],
            ),
            ItemSlot::Player { .. } | ItemSlot::Stored { .. } => {
                let mut arguments = vec![NbtSummaryValue::Int(count)];
                if !modifying {
                    arguments.push(access.display(stack)?);
                }
                translatable("commands.item.entity.set.success.multiple", arguments)
            }
        };
        Ok((count, feedback))
    }

    fn modified(
        &self,
        access: &mut WorldItemAccess<'_>,
        stack: RuntimeItemStack,
    ) -> Result<RuntimeItemStack> {
        match &self.modifier {
            Some(modifier) => access.modify(modifier, stack),
            None => Ok(stack),
        }
    }
}

fn slot_number(slot: &ItemSlot) -> i32 {
    match slot {
        ItemSlot::Block { slot, .. }
        | ItemSlot::Player { slot, .. }
        | ItemSlot::Stored { slot, .. } => *slot,
    }
}

fn no_slot(slot: &ItemSlot, target: bool) -> NbtSummaryValue {
    translatable(
        if target {
            "commands.item.target.no_such_slot"
        } else {
            "commands.item.source.no_such_slot"
        },
        vec![NbtSummaryValue::Int(slot_number(slot))],
    )
}

pub(crate) fn container_slot(
    size: Option<usize>,
    pos: BlockPos,
    slot: i32,
    target: bool,
) -> Result<()> {
    let size = size.ok_or_else(|| {
        translatable(
            if target {
                "commands.item.target.not_a_container"
            } else {
                "commands.item.source.not_a_container"
            },
            vec![
                NbtSummaryValue::Int(pos.x),
                NbtSummaryValue::Int(pos.y),
                NbtSummaryValue::Int(pos.z),
            ],
        )
    })?;
    usize::try_from(slot)
        .ok()
        .filter(|index| *index < size)
        .map(|_| ())
        .ok_or_else(|| no_slot(&ItemSlot::Block { pos, slot }, target))
}

fn finish(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    input: ItemInput,
    targets: Vec<ItemSlot>,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let modifier = match parse.argument("modifier") {
        Some(ArgumentValue::LootModifier(modifier)) => Some(modifier.clone()),
        None => None,
        other => unreachable!("modifier is a loot modifier, not {other:?}"),
    };
    let command = ItemCommand {
        level: cx.origin.level.clone(),
        origin: cx.origin.position,
        source: source.clone(),
        input,
        targets,
        modifier,
    };
    let (count, feedback) = cx.world.run_item_command(&command, cx.events)?;
    run.succeed(feedback, true);
    Ok(count)
}

pub(super) fn set_block_item(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (world, at): (&ServerWorld, &CommandOrigin) = (cx.world, &cx.origin);
    let pos = loaded_block_pos(parse, "pos", world.command_blocks(&at.level), at)?;
    let stack = counted_item_stack(parse, world)?;
    finish(
        run,
        parse,
        source,
        ItemInput::Stack(stack),
        vec![block_slot(parse, "slot", pos)],
        cx,
    )
}

pub(super) fn block_to_block(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (world, at): (&ServerWorld, &CommandOrigin) = (cx.world, &cx.origin);
    let source_pos = loaded_block_pos(parse, "source", world.command_blocks(&at.level), at)?;
    let target = loaded_block_pos(parse, "pos", world.command_blocks(&at.level), at)?;
    finish(
        run,
        parse,
        source,
        ItemInput::Slot(block_slot(parse, "sourceSlot", source_pos)),
        vec![block_slot(parse, "slot", target)],
        cx,
    )
}

pub(super) fn modify_block_item(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (world, at): (&ServerWorld, &CommandOrigin) = (cx.world, &cx.origin);
    let target = loaded_block_pos(parse, "pos", world.command_blocks(&at.level), at)?;
    finish(
        run,
        parse,
        source,
        ItemInput::Modify,
        vec![block_slot(parse, "slot", target)],
        cx,
    )
}

pub(super) fn modify_entity_item(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let at = &cx.origin;
    let targets = cx.world.with_entity_reads(|reads| {
        Ok::<_, NbtSummaryValue>(entity_slots(
            parse,
            damage_entity_targets(parse, "targets", source, at, reads)?,
            reads,
        ))
    })?;
    finish(run, parse, source, ItemInput::Modify, targets, cx)
}

pub(super) fn set_entity_item(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let at = &cx.origin;
    let targets = cx.world.with_entity_reads(|reads| {
        Ok::<_, NbtSummaryValue>(entity_slots(
            parse,
            damage_entity_targets(parse, "targets", source, at, reads)?,
            reads,
        ))
    })?;
    let stack = counted_item_stack(parse, cx.world)?;
    finish(run, parse, source, ItemInput::Stack(stack), targets, cx)
}

pub(super) fn block_to_entities(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let at = &cx.origin;
    let pos = loaded_block_pos(parse, "source", cx.world.command_blocks(&at.level), at)?;
    let targets = cx.world.with_entity_reads(|reads| {
        Ok::<_, NbtSummaryValue>(entity_slots(
            parse,
            damage_entity_targets(parse, "targets", source, at, reads)?,
            reads,
        ))
    })?;
    finish(
        run,
        parse,
        source,
        ItemInput::Slot(block_slot(parse, "sourceSlot", pos)),
        targets,
        cx,
    )
}

pub(super) fn entity_to_block(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let at = &cx.origin;
    let player = cx.world.with_entity_reads(|reads| {
        Ok::<_, NbtSummaryValue>(entity_slot_input(
            parse,
            single_damage_entity_argument(parse, "source", source, at, reads)?,
            reads,
        ))
    })?;
    let target = loaded_block_pos(parse, "pos", cx.world.command_blocks(&at.level), at)?;
    finish(
        run,
        parse,
        source,
        ItemInput::Slot(player),
        vec![block_slot(parse, "slot", target)],
        cx,
    )
}

pub(super) fn entity_to_entities(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let at = &cx.origin;
    let (player, targets) = cx.world.with_entity_reads(|reads| {
        let player = entity_slot_input(
            parse,
            single_damage_entity_argument(parse, "source", source, at, reads)?,
            reads,
        );
        let targets = entity_slots(
            parse,
            damage_entity_targets(parse, "targets", source, at, reads)?,
            reads,
        );
        Ok::<_, NbtSummaryValue>((player, targets))
    })?;
    finish(run, parse, source, ItemInput::Slot(player), targets, cx)
}

fn block_slot(parse: &ParseResult, name: &str, pos: BlockPos) -> ItemSlot {
    ItemSlot::Block {
        pos,
        slot: named_slot(parse, name),
    }
}
fn entity_slots(
    parse: &ParseResult,
    entities: Vec<CommandDamageEntityRef<'_>>,
    reads: &crate::world::EntityReads<'_>,
) -> Vec<ItemSlot> {
    entities
        .into_iter()
        .map(|entity| entity_slot(entity, named_slot(parse, "slot"), reads))
        .collect()
}
fn entity_slot_input(
    parse: &ParseResult,
    entity: CommandDamageEntityRef<'_>,
    reads: &crate::world::EntityReads<'_>,
) -> ItemSlot {
    entity_slot(entity, named_slot(parse, "sourceSlot"), reads)
}
pub(super) fn entity_slot(
    entity: CommandDamageEntityRef<'_>,
    slot: i32,
    reads: &crate::world::EntityReads<'_>,
) -> ItemSlot {
    match entity {
        CommandDamageEntityRef::Player(player) => ItemSlot::Player {
            player: player.target(),
            slot,
        },
        CommandDamageEntityRef::Stored(stored) => ItemSlot::Stored {
            uuid: stored.entity.uuid(),
            level: stored.dimension.clone(),
            name: entity.display_name(reads),
            slot,
        },
    }
}

// ItemCommands.java:490-496; ItemStack.java:580-587: copy canonicalizes public empty.
fn copied_stack(stack: RuntimeItemStack) -> RuntimeItemStack {
    if stack.is_empty() {
        RuntimeItemStack::empty()
    } else {
        stack
    }
}

fn counted_item_stack(parse: &ParseResult, world: &ServerWorld) -> Result<RuntimeItemStack> {
    let (_, resource_id, patch) = item_stack_argument(parse);
    let count = parse
        .argument("count")
        .map(|_| integer_argument(parse, "count"))
        .unwrap_or(1);
    let resolver = world.item_stack_resolver();
    native_item_argument_stack(&resource_id, &patch, count, true, resolver.as_dyn())
}

pub(crate) fn player_slot_accepts(slot: i32, stack: &RuntimeItemStack) -> Result<bool> {
    use bbb_protocol::equipment_slots::EquipmentSlot;
    match player_slot(slot, 0) {
        Some(PlayerSlot::Cursor | PlayerSlot::Crafting(_) | PlayerSlot::EnderChest(_)) => {
            return Ok(true);
        }
        Some(PlayerSlot::Inventory(_)) if (0..36).contains(&slot) => return Ok(true),
        Some(PlayerSlot::Inventory(_)) => {}
        None => return Ok(false),
    }
    let Some(equipment) = equipment_slot_for_command_slot(slot) else {
        return Ok(false);
    };
    match equipment {
        EquipmentSlot::Head | EquipmentSlot::MainHand | EquipmentSlot::OffHand => Ok(true),
        other => Ok(stack.is_empty() || stack.equippable_slot().map_err(failure)? == Some(other)),
    }
}

fn named_slot(parse: &ParseResult, name: &str) -> i32 {
    match parse.argument(name) {
        Some(ArgumentValue::ItemSlot(slot)) => *slot,
        other => unreachable!("{name} is an item slot, not {other:?}"),
    }
}
fn failure(error: impl std::fmt::Display) -> NbtSummaryValue {
    bbb_protocol::component_literal_nbt(error.to_string())
}
