//! `/loot` sources and delivery through the existing stack and container owners.

use bbb_protocol::NbtSummaryValue;

use crate::commands::loot_table::CommandLootTable;
use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandCx, CommandPlayer, CommandSource};

use super::{player_states_argument, runtime_item_stack_display_name, CommandRun, Outcome};

pub(super) fn drop_loot(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let arm = parse
        .nodes
        .iter()
        .rev()
        .map(|node| crate::commands::catalog::node(node.index).name)
        .find(|name| matches!(*name, "loot" | "fish" | "kill" | "mine"))
        .expect("loot source literal");
    let (drops, table) = match arm {
        "loot" | "fish" => {
            let fishing = arm == "fish";
            let origin = if fishing {
                let pos = super::blocks::loaded_block_pos(
                    parse,
                    "pos",
                    cx.world.command_blocks(&cx.origin.level),
                    &cx.origin,
                )?;
                [
                    f64::from(pos.x) + 0.5,
                    f64::from(pos.y) + 0.5,
                    f64::from(pos.z) + 0.5,
                ]
            } else {
                cx.origin.position
            };
            let tool = fishing
                .then(|| source_tool(parse, source, cx))
                .transpose()?;
            (
                cx.world.roll_command_loot_table(
                    loot_table_argument(parse),
                    &cx.origin.level,
                    origin,
                    source,
                    tool.as_ref(),
                )?,
                None,
            )
        }
        "kill" => {
            let (table, target) = cx.world.with_entity_reads(|reads| {
                let target = super::single_damage_entity_argument(
                    parse, "target", source, &cx.origin, reads,
                )?;
                let table = match target {
                    super::CommandDamageEntityRef::Player(_) => {
                        bbb_protocol::entity_types::vanilla_entity_type(
                            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
                        )
                        .and_then(|kind| kind.default_loot_table)
                    }
                    super::CommandDamageEntityRef::Stored(stored) => {
                        crate::entity::mob_death_loot::entity_loot_table_with_registry(
                            stored.entity,
                            reads.content_registry(),
                        )
                    }
                }
                .ok_or_else(|| {
                    translatable(
                        "commands.drop.no_loot_table.entity",
                        vec![target.display_name(reads)],
                    )
                })?
                .to_owned();
                let target = super::command_entity_source(target, source.permission, reads);
                Ok::<_, NbtSummaryValue>((table, target))
            })?;
            (
                cx.world.roll_command_kill_loot(
                    &table,
                    &target,
                    &cx.origin.level,
                    cx.origin.position,
                    source,
                )?,
                Some(table),
            )
        }
        "mine" => {
            let pos = super::blocks::loaded_block_pos(
                parse,
                "pos",
                cx.world.command_blocks(&cx.origin.level),
                &cx.origin,
            )?;
            let tool = source_tool(parse, source, cx)?;
            let (table, drops) =
                cx.world
                    .roll_command_mine_loot(&cx.origin.level, pos, source, &tool)?;
            (drops, Some(table))
        }
        _ => unreachable!("loot source"),
    };
    let root = parse
        .nodes
        .iter()
        .position(|node| {
            let literal = crate::commands::catalog::node(node.index);
            literal.name == "loot"
                && literal.parent == Some(bbb_protocol::packets::VANILLA_COMMAND_TREE_ROOT)
        })
        .expect("loot command root");
    let destination = crate::commands::catalog::node(parse.nodes[root + 1].index).name;
    let mut used = Vec::new();
    match destination {
        "spawn" => {
            let pos = super::coordinates_argument(parse, "targetPos").position(&cx.origin);
            for stack in drops {
                cx.world
                    .spawn_command_loot(&cx.origin.level, pos, stack.clone());
                used.push(stack);
            }
        }
        "give" => {
            let players = cx.world.with_entity_reads(|reads| {
                Ok::<_, NbtSummaryValue>(
                    player_states_argument(parse, "players", source, reads, &cx.origin)?
                        .iter()
                        .map(CommandPlayer::target)
                        .collect::<Vec<_>>(),
                )
            })?;
            for stack in drops {
                for player in &players {
                    if cx.world.add_command_loot(player, &stack)? {
                        used.push(stack.clone());
                    }
                }
            }
        }
        "insert" | "replace"
            if destination == "insert"
                || crate::commands::catalog::node(parse.nodes[root + 2].index).name == "block" =>
        {
            let pos = super::blocks::loaded_block_pos(
                parse,
                "targetPos",
                cx.world.command_blocks(&cx.origin.level),
                &cx.origin,
            )?;
            let replace = (destination == "replace")
                .then(|| (slot_argument(parse), replacement_count(parse, drops.len())));
            used =
                cx.world
                    .distribute_command_block_loot(&cx.origin.level, pos, replace, &drops)?;
        }
        "replace" => {
            let start = slot_argument(parse);
            let count = replacement_count(parse, drops.len());
            let mut slots = bbb_server_types::slot_ranges::single_slot_names()
                .iter()
                .filter_map(|name| bbb_server_types::slot_ranges::parse_single_slot(name).ok())
                .filter(|slot| *slot >= start && ((*slot - start) as usize) < count)
                .collect::<Vec<_>>();
            slots.sort_unstable();
            slots.dedup();
            let targets = cx.world.with_entity_reads(|reads| {
                Ok::<_, NbtSummaryValue>(
                    super::damage_entity_targets(parse, "entities", source, &cx.origin, reads)?
                        .into_iter()
                        .map(|entity| {
                            slots
                                .iter()
                                .map(|slot| {
                                    (
                                        (*slot - start) as usize,
                                        super::item::entity_slot(entity, *slot, reads),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>(),
                )
            })?;
            for target in targets {
                for (offset, slot) in target {
                    if cx
                        .world
                        .read_command_item_slot(&cx.origin.level, &slot, None)?
                        .is_none()
                    {
                        continue;
                    }
                    let stack = drops.get(offset).cloned().unwrap_or_default();
                    if cx.world.write_command_item_slot(
                        &cx.origin.level,
                        &slot,
                        stack.clone(),
                        cx.events,
                    )? {
                        used.push(stack);
                    }
                }
            }
        }
        _ => unreachable!("loot destination"),
    }
    let count = used.len() as i32;
    let mut arguments = if used.len() == 1 {
        let stack = &used[0];
        vec![
            NbtSummaryValue::Int(stack.count()),
            runtime_item_stack_display_name(stack, &cx.world.item_stack_resolver())?,
        ]
    } else {
        vec![NbtSummaryValue::Int(count)]
    };
    let key = match (used.len() == 1, table) {
        (single, Some(table)) => {
            arguments.push(bbb_protocol::component_literal_nbt(table));
            if single {
                "commands.drop.success.single_with_table"
            } else {
                "commands.drop.success.multiple_with_table"
            }
        }
        (true, None) => "commands.drop.success.single",
        (false, None) => "commands.drop.success.multiple",
    };
    run.succeed(translatable(key, arguments), false);
    Ok(count)
}

fn slot_argument(parse: &ParseResult) -> i32 {
    match parse.argument("slot") {
        Some(ArgumentValue::ItemSlot(slot)) => *slot,
        _ => unreachable!("loot slot"),
    }
}

fn replacement_count(parse: &ParseResult, default: usize) -> usize {
    parse.argument("count").map_or(default, |_| {
        super::integer_argument(parse, "count") as usize
    })
}

fn source_tool(
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Result<bbb_server_types::item_stack::RuntimeItemStack, NbtSummaryValue> {
    use bbb_server_types::item_stack::RuntimeItemStack;
    if let Some(ArgumentValue::ItemStack {
        resource_id, patch, ..
    }) = parse.argument("tool")
    {
        return crate::commands::item_modifier::native_item_argument_stack(
            resource_id,
            patch,
            1,
            false,
            &cx.world.item_stack_resolver(),
        );
    }
    let hand = parse
        .nodes
        .last()
        .map(|node| crate::commands::catalog::node(node.index).name)
        .filter(|name| matches!(*name, "mainhand" | "offhand"));
    let Some(hand) = hand else {
        return Ok(RuntimeItemStack::empty());
    };
    let reads = cx.world.entity_reads();
    let entity = super::source_entity(source, &reads)?;
    let slot = bbb_server_types::slot_ranges::parse_single_slot(&format!("weapon.{hand}"))
        .expect("hand slot");
    let slot = match entity {
        crate::commands::types::CommandDamageEntityRef::Player(player) => {
            crate::commands::ItemSlot::Player {
                player: player.target(),
                slot,
            }
        }
        crate::commands::types::CommandDamageEntityRef::Stored(stored) => {
            if stored.entity.living().is_none() {
                return Err(translatable(
                    "commands.drop.no_held_items",
                    vec![entity.display_name(&reads)],
                ));
            }
            crate::commands::ItemSlot::Stored {
                uuid: stored.entity.uuid(),
                level: stored.dimension.clone(),
                name: entity.display_name(&reads),
                slot,
            }
        }
    };
    cx.world
        .read_command_item_slot(&cx.origin.level, &slot, Some(false))?
        .ok_or_else(|| translatable("permissions.requires.entity", Vec::new()))
}

fn loot_table_argument(parse: &ParseResult) -> &CommandLootTable {
    match parse.argument("loot_table") {
        Some(ArgumentValue::LootTable(table)) => table,
        other => unreachable!("loot_table is a loot_table argument, not {other:?}"),
    }
}

pub(super) fn predicate_holds(
    predicate: &crate::commands::loot_predicate::CommandLootPredicate,
    source: &CommandSource,
    world: &crate::world::EntityReads<'_>,
    at: &crate::commands::CommandOrigin,
) -> Result<bool, NbtSummaryValue> {
    use crate::commands::types::CommandDamageEntityRef;
    use crate::loot_runtime::{
        matches_conditions, param_sets, LootBlockEntityDataAt, LootBlockStateAt, LootEntities,
        LootEntity, LootOrigin, LootParams, LootRuntimeReason,
    };
    use bbb_server_datapack::loot::LootRegistries;

    let empty = LootRegistries {
        registries: Vec::new(),
        warnings: Vec::new(),
        tag_warnings: Vec::new(),
    };
    let registries = world.loot.unwrap_or(&empty);
    let facts;
    let this_entity = match super::source_entity_state(source, world) {
        Some(CommandDamageEntityRef::Player(player)) => {
            facts = world.player_loot_facts(player);
            LootEntity::Facts(&facts)
        }
        Some(CommandDamageEntityRef::Stored(stored)) => {
            LootEntity::Stored(world.stored_loot_entity(stored))
        }
        None => LootEntity::Absent,
    };
    let loaded_block_state = world.blocks(&at.level).map(|_| {
        move |pos| {
            let blocks = world.blocks(&at.level).expect("served predicate chunks");
            (blocks.is_in_world_bounds(pos) && blocks.has_chunk_at(pos))
                .then(|| blocks.block_state_at(pos))
        }
    });
    let block_entity_data = world.blocks(&at.level).map(|_| {
        move |pos| {
            let blocks = world.blocks(&at.level).expect("served predicate chunks");
            blocks
                .block_entity_loot_data(pos, Some(world.resolver()))
                .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
        }
    });
    let player_slots = |uuid, slot| world.player_item_slot(uuid, slot);
    let player_nbt = |uuid| world.player_loot_nbt(uuid);
    let item_environment = world.predicate_environment(&at.level, &player_slots, &player_nbt);
    let vehicle_slots = |requested: &crate::entity::Entity, selected: &[i32]| {
        world.loot_vehicle_slots(requested, selected)
    };
    let [x, y, z] = at.position;
    let params = LootParams {
        dimension: Some(at.level.as_str()),
        item_environment: match &item_environment {
            Some(environment) => Some(environment),
            None => None,
        },
        origin: Some(LootOrigin { x, y, z }),
        entities: LootEntities {
            this_entity,
            ..LootEntities::default()
        },
        weather: Some(world.weather(&at.level)),
        vehicle_slots: Some(&vehicle_slots),
        level: loaded_block_state
            .as_ref()
            .map(|read| read as &LootBlockStateAt<'_>),
        block_entity_data: block_entity_data
            .as_ref()
            .map(|read| read as &LootBlockEntityDataAt<'_>),
        ..LootParams::new(param_sets::COMMAND)
    };
    matches_conditions(
        registries,
        &predicate.id,
        std::slice::from_ref(&predicate.condition),
        &params,
        world.random(&at.level),
    )
    .map_err(|error| {
        super::styled_translatable(
            "command.failed",
            Vec::new(),
            &bbb_protocol::ComponentStyle {
                hover_event: Some(super::scoreboard::show_text(error.to_string())),
                ..Default::default()
            },
        )
    })
}
