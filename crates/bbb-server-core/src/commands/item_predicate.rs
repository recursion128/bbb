//! Parsed command item predicates over the shared loot predicate evaluator.

use bbb_protocol::{
    component_literal_nbt, scan_command_item_predicate_head_utf16,
    CommandItemPredicateTestScannerUtf16, CommandSyntaxErrorUtf16, NbtSummaryEntry,
    NbtSummaryValue, PersistentRegistryTagMembers, ScannedCommandItemPredicateHeadUtf16,
    ScannedCommandItemPredicateTestUtf16,
};
use bbb_registry_codec::{validate_persistent_component_nbt, PersistentItemStackHolderResolver};
use bbb_server_datapack::loot::{
    parse_item_predicate_nbt, ComponentPredicate, HolderSet, ItemPredicate,
};

use super::item_modifier::persistent_decode_context;
use super::parse::ParseFailure;
use super::translatable;
use crate::loot_runtime::item_predicate::item_predicate_matches;
use crate::loot_runtime::LootTool;
use crate::world::ServerWorld;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CommandItemPredicate {
    head: ItemPredicate,
    groups: Vec<Vec<(bool, ItemPredicate)>>,
}

impl CommandItemPredicate {
    pub(super) fn matches(
        &self,
        tool: &LootTool,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<bool, crate::loot_runtime::LootRuntimeReason> {
        let matches = |predicate| item_predicate_matches(predicate, tool, resolver);
        if !matches(&self.head)? {
            return Ok(false);
        }
        for group in &self.groups {
            let mut found = false;
            for (negated, predicate) in group {
                if matches(predicate)? != *negated {
                    found = true;
                    break;
                }
            }
            if !found {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

pub(super) fn read(
    command: &[u16],
    cursor: usize,
    world: &ServerWorld,
) -> Result<(usize, CommandItemPredicate), ParseFailure> {
    let head = scan_command_item_predicate_head_utf16(command, cursor).map_err(syntax_failure)?;
    let resolver = world.item_stack_resolver();
    let items = resolve_head(&head, &resolver)?;
    let mut predicate = CommandItemPredicate {
        head: ItemPredicate {
            items,
            ..ItemPredicate::default()
        },
        groups: Vec::new(),
    };
    let mut scanner = CommandItemPredicateTestScannerUtf16::new(command, head.end());
    loop {
        let term = match scanner.next_term() {
            Ok(Some(term)) => term,
            Ok(None) => break,
            Err(error) => {
                return Err(scanner
                    .take_snbt_failure()
                    .and_then(|failure| super::parse::snbt_failure(0, &failure))
                    .unwrap_or_else(|| syntax_failure(error)));
            }
        };
        let atom = compile_test(term.test, &resolver)?;
        if !term.alternative {
            predicate.groups.push(Vec::new());
        }
        predicate
            .groups
            .last_mut()
            .expect("a group starts with its first term")
            .push((term.negated, atom));
    }
    Ok((scanner.end(), predicate))
}

pub(super) fn resolve_head(
    head: &ScannedCommandItemPredicateHeadUtf16,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<Option<HolderSet>, ParseFailure> {
    Ok(match head {
        ScannedCommandItemPredicateHeadUtf16::Any { .. } => None,
        ScannedCommandItemPredicateHeadUtf16::Item {
            resource_id, end, ..
        } => {
            if bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id).is_none()
                && resolver
                    .content_registry()
                    .item_by_resource_id(resource_id)
                    .is_none()
            {
                return Err(failure("argument.item.id.invalid", resource_id, *end));
            }
            Some(HolderSet::Ids(vec![resource_id.clone()]))
        }
        ScannedCommandItemPredicateHeadUtf16::Tag {
            resource_id, end, ..
        } => {
            if !matches!(
                resolver.tag_members("minecraft:item", resource_id),
                PersistentRegistryTagMembers::Present(_)
            ) {
                return Err(failure("arguments.item.tag.unknown", resource_id, *end));
            }
            Some(HolderSet::Tag(resource_id.clone()))
        }
    })
}

pub(super) fn compile_test(
    test: ScannedCommandItemPredicateTestUtf16,
    resolver: &crate::world::registry_runtime::PlayerItemStackResolver,
) -> Result<ItemPredicate, ParseFailure> {
    Ok(match test {
        ScannedCommandItemPredicateTestUtf16::Presence { resource_id, .. } => {
            let mut atom = ItemPredicate::default();
            if resource_id != "minecraft:count" {
                atom.predicates
                    .push(ComponentPredicate::AnyValue(resource_id));
            }
            atom
        }
        ScannedCommandItemPredicateTestUtf16::Exact {
            resource_id,
            component,
            value,
            value_end,
            ..
        } => {
            if let Some(component) = component {
                validate_persistent_component_nbt(
                    component.type_id,
                    &resource_id,
                    &value,
                    persistent_decode_context(resolver),
                )
                .map_err(|error| malformed(false, &resource_id, error.to_string(), value_end))?;
                ItemPredicate {
                    components: vec![(resource_id, value)],
                    ..ItemPredicate::default()
                }
            } else {
                parse_atom("count", value, false, &resource_id, value_end)?
            }
        }
        ScannedCommandItemPredicateTestUtf16::Subpredicate {
            resource_id,
            value,
            value_end,
            ..
        } => {
            if resource_id == "minecraft:count" {
                parse_atom("count", value, true, &resource_id, value_end)?
            } else {
                parse_atom(
                    "predicates",
                    compound(&resource_id, value),
                    true,
                    &resource_id,
                    value_end,
                )?
            }
        }
    })
}

fn compound(name: &str, value: NbtSummaryValue) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: name.into(),
        value,
    }])
}

fn parse_atom(
    field: &str,
    value: NbtSummaryValue,
    subpredicate: bool,
    id: &str,
    cursor: usize,
) -> Result<ItemPredicate, ParseFailure> {
    parse_item_predicate_nbt(&compound(field, value))
        .map_err(|error| malformed(subpredicate, id, error.to_string(), cursor))
}

fn malformed(
    subpredicate: bool,
    id: &str,
    message: impl Into<String>,
    cursor: usize,
) -> ParseFailure {
    ParseFailure {
        message: translatable(
            if subpredicate {
                "arguments.item.predicate.malformed"
            } else {
                "arguments.item.component.malformed"
            },
            vec![
                NbtSummaryValue::String(id.into()),
                NbtSummaryValue::String(message.into().into()),
            ],
        ),
        cursor: Some(cursor),
    }
}

fn failure(key: &str, id: impl Into<bbb_protocol::JavaString>, cursor: usize) -> ParseFailure {
    ParseFailure {
        message: translatable(key, vec![component_literal_nbt(id)]),
        cursor: Some(cursor),
    }
}

fn syntax_failure(error: CommandSyntaxErrorUtf16) -> ParseFailure {
    let cursor = error.cursor;
    if let Some(id) = error
        .message
        .units()
        .get(..23)
        .filter(|units| {
            "unknown item component "
                .encode_utf16()
                .eq(units.iter().copied())
        })
        .map(|_| &error.message.units()[23..])
    {
        return failure("arguments.item.component.unknown", id, cursor);
    }
    ParseFailure {
        message: match error.message {
            message if message == "invalid resource location" => {
                translatable("argument.id.invalid", Vec::new())
            }
            message if message == "expected item sub-predicate assignment" => {
                component_literal_nbt("Expected '~'")
            }
            message if message == "expected item component assignment" => {
                component_literal_nbt("Expected '='")
            }
            message if message == "expected item predicate separator or end" => {
                component_literal_nbt("Expected ']'")
            }
            other => component_literal_nbt(other),
        },
        cursor: Some(cursor),
    }
}
