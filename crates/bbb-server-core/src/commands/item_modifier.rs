//! Command item inputs and component edits over held Native stacks.

use bbb_protocol::{
    component_literal_nbt, scan_command_resource_or_id_utf16,
    CommandItemComponentPatchScannerUtf16, CommandSyntaxErrorUtf16, NbtSummaryEntry,
    NbtSummaryValue, ScannedCommandResourceOrIdUtf16,
};
use bbb_registry_codec::{
    validate_persistent_component_nbt, PersistentItemStackHolderResolver,
    PersistentNbtDecodeContext,
};
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::{
    parse_loot_function_nbt, LootElementValue, LootEntry, LootEntryKind, LootFunction,
    LootFunctionKind,
};

use super::parse::ParseFailure;
use super::translatable;
use crate::world::registry_runtime::PlayerItemStackResolver;
use crate::world::ServerWorld;

/// The registry `ResourceOrIdArgument.LootModifierArgument` names
/// (`ResourceOrIdArgument.java:166-170`).
pub(super) const ITEM_MODIFIER_REGISTRY: &str = "minecraft:item_modifier";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CommandItemModifier {
    pub(crate) id: String,
    pub(crate) function: LootFunction,
}

/// One entry of a `DataComponentPatch.CODEC` value
/// (`SetComponentsFunction.java:20-24`): a component to set, or -- with the
/// `!` prefix `DataComponentPatch.CODEC` reads -- one to remove.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CommandComponentPatchEntry {
    Set {
        resource_id: String,
        value: NbtSummaryValue,
    },
    Remove {
        resource_id: String,
    },
}

/// `ResourceOrIdArgument.parse` (`ResourceOrIdArgument.java:123-132`) for
/// `Registries.ITEM_MODIFIER`: a name resolves against the live registry and
/// anything else is one inline SNBT value through `LootItemFunctions.ROOT_CODEC`.
pub(super) fn read_loot_modifier(
    command: &[u16],
    cursor: usize,
    world: &ServerWorld,
) -> Result<(usize, CommandItemModifier), ParseFailure> {
    let scanned = scan_command_resource_or_id_utf16(command, cursor)
        .map_err(|_| failed_to_parse(cursor, "expected an id or an inline SNBT value"))?;
    match scanned {
        ScannedCommandResourceOrIdUtf16::Reference { end, resource_id } => {
            // `ReferenceResult.parse` (`ResourceOrIdArgument.java:184-196`).
            let found = world
                .loot_elements("minecraft:item_modifier")
                .iter()
                .find_map(
                    |element| match (&element.value, element.id == resource_id) {
                        (LootElementValue::Modifier(function), true) => Some(function),
                        _ => None,
                    },
                );
            let Some(function) = found else {
                return Err(ParseFailure {
                    message: translatable(
                        "argument.resource_or_id.no_such_element",
                        vec![
                            NbtSummaryValue::string(resource_id),
                            NbtSummaryValue::string(ITEM_MODIFIER_REGISTRY),
                        ],
                    ),
                    cursor: Some(cursor),
                });
            };
            validate_component_values(function, &world.item_stack_resolver())
                .map_err(|message| failed_to_parse(cursor, message))?;
            let modifier = CommandItemModifier {
                id: resource_id,
                function: function.clone(),
            };
            Ok((end, modifier))
        }
        // `InlineResult.parse` (`ResourceOrIdArgument.java:150-163`) decodes the
        // scanned value with `NbtOps`, so a byte reads as the boolean a
        // `Codec.BOOL` field wants.
        ScannedCommandResourceOrIdUtf16::Inline { end, value } => {
            let lookup = world
                .datapacks()
                .map(FrozenDatapackRegistryLookup::from_startup);
            let function = parse_loot_function_nbt(&value, lookup.as_ref())
                .map_err(|error| failed_to_parse(cursor, error.to_string()))?;
            validate_modifier(&function, world)
                .map_err(|message| failed_to_parse(cursor, message))?;
            let modifier = CommandItemModifier {
                id: String::new(),
                function,
            };
            Ok((end, modifier))
        }
    }
}

/// `ERROR_FAILED_TO_PARSE` (`ResourceOrIdArgument.java:46-48`), which
/// `InlineResult.parse` raises with the codec's own message (`:161`).
fn failed_to_parse(cursor: usize, message: impl Into<String>) -> ParseFailure {
    ParseFailure {
        message: translatable(
            "argument.resource_or_id.failed_to_parse",
            vec![NbtSummaryValue::String(message.into().into())],
        ),
        cursor: Some(cursor),
    }
}

fn validate_modifier(function: &LootFunction, world: &ServerWorld) -> Result<(), String> {
    world
        .check_item_modifier(function)
        .map_err(|error| error.to_string())?;
    validate_component_values(function, &world.item_stack_resolver())
}

/// `DataComponentPatch.CODEC` over every `set_components` map the modifier
/// reaches, against the player's registries; the codec's own message is the
/// parse failure.
fn validate_component_values(
    function: &LootFunction,
    resolver: &PlayerItemStackResolver,
) -> Result<(), String> {
    match &function.kind {
        LootFunctionKind::SetComponents(components) => {
            crate::loot_runtime::prepare_component_patch(components, resolver).map_err(
                |error| match error {
                    crate::loot_runtime::LootRuntimeReason::Catalog(detail) => detail,
                    other => other.to_string(),
                },
            )?;
        }
        LootFunctionKind::Sequence(functions) => {
            for nested in functions {
                validate_component_values(nested, resolver)?;
            }
        }
        LootFunctionKind::Filtered {
            on_pass, on_fail, ..
        } => {
            for nested in [on_pass, on_fail].into_iter().flatten() {
                validate_component_values(nested, resolver)?;
            }
        }
        LootFunctionKind::ModifyContents { modifier, .. } => {
            validate_component_values(modifier, resolver)?;
        }
        LootFunctionKind::SetContents { entries, .. } => {
            for entry in entries {
                validate_entry_component_values(entry, resolver)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_entry_component_values(
    entry: &LootEntry,
    resolver: &PlayerItemStackResolver,
) -> Result<(), String> {
    for function in &entry.functions {
        validate_component_values(function, resolver)?;
    }
    if let LootEntryKind::Alternatives(children)
    | LootEntryKind::Group(children)
    | LootEntryKind::Sequence(children) = &entry.kind
    {
        for child in children {
            validate_entry_component_values(child, resolver)?;
        }
    }
    Ok(())
}
/// `ItemParser.readComponents` (`ItemParser.java:143-191`): the `[...]` patch
/// `ItemParser.State.parse` (`:124-132`) reads only when the next character is
/// `[` (`:128-131`).
///
/// The grammar layer hands one entry over at a time because vanilla decodes
/// each value the moment it has read it (`:209-217`): a list holding both a
/// rejected value and a later syntax fault reports the value. Each entry is
/// validated by the per-component persistent decoder; `/item modify
/// set_components` decodes its whole patch through the Native component patch
/// decoder instead, so the two surfaces word their refusals differently.
pub(super) fn read_item_component_patch(
    command: &[u16],
    start: usize,
    world: &ServerWorld,
) -> Result<(usize, Vec<CommandComponentPatchEntry>), ParseFailure> {
    let resolver = world.item_stack_resolver();
    let context = persistent_decode_context(&resolver);
    let mut scanner = CommandItemComponentPatchScannerUtf16::new(command, start);
    let mut entries = Vec::new();
    loop {
        let scanned = match scanner.next_component() {
            Ok(Some(scanned)) => scanned,
            Ok(None) => break,
            Err(error) => {
                // `readComponent` (`:209-217`) lets `parseAsArgument` throw
                // through, so a value the grammar refuses reports its own
                // farthest failure and only the codec result is rewrapped.
                let failure = scanner
                    .take_snbt_failure()
                    .and_then(|failure| super::parse::snbt_failure(0, &failure));
                return Err(failure.unwrap_or_else(|| item_patch_failure(command, error)));
            }
        };
        if scanned.removed {
            entries.push(CommandComponentPatchEntry::Remove {
                resource_id: scanned.component.resource_id.to_string(),
            });
            continue;
        }
        let value = scanned
            .value
            .expect("an assigned component carries a value");
        let value_start = scanned
            .value_start
            .expect("an assigned component carries a value cursor");
        validate_persistent_component_nbt(
            scanned.component.type_id,
            scanned.component.resource_id,
            &value,
            context,
        )
        .map_err(|error| ParseFailure {
            // `ERROR_MALFORMED_COMPONENT` (`:41-43`, thrown at `:213-216`),
            // whose first argument is `componentType.toString()` -- the
            // registered name (`DataComponentType.java:112-114`).
            message: translatable(
                "arguments.item.component.malformed",
                vec![
                    NbtSummaryValue::string(scanned.component.resource_id),
                    NbtSummaryValue::string(error.to_string()),
                ],
            ),
            cursor: Some(value_start),
        })?;
        entries.push(CommandComponentPatchEntry::Set {
            resource_id: scanned.component.resource_id.to_string(),
            value,
        });
    }
    Ok((scanner.end(), entries))
}

fn command_patch_nbt(patch: &[CommandComponentPatchEntry]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        patch
            .iter()
            .map(|entry| match entry {
                CommandComponentPatchEntry::Set { resource_id, value } => NbtSummaryEntry {
                    name: resource_id.clone().into(),
                    value: value.clone(),
                },
                CommandComponentPatchEntry::Remove { resource_id } => NbtSummaryEntry {
                    name: format!("!{resource_id}").into(),
                    value: NbtSummaryValue::Compound(Vec::new()),
                },
            })
            .collect(),
    )
}

pub(super) fn native_item_argument_prototype(
    resource_id: &str,
    patch: &[CommandComponentPatchEntry],
    resolver: &PlayerItemStackResolver,
) -> Result<bbb_server_types::item_stack::RuntimeItemStack, NbtSummaryValue> {
    native_item_argument_stack(resource_id, patch, 1, false, resolver)
}

pub(super) fn native_item_argument_stack(
    resource_id: &str,
    patch: &[CommandComponentPatchEntry],
    count: i32,
    check_overstack: bool,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<bbb_server_types::item_stack::RuntimeItemStack, NbtSummaryValue> {
    let stack = if resource_id == "minecraft:air" {
        bbb_server_types::item_stack::RuntimeItemStack::empty()
    } else {
        let raw = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String(resource_id.into()),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: command_patch_nbt(patch),
            },
        ]);
        bbb_server_types::item_stack::RuntimeItemStack::try_read(&raw, resolver)
            .map_err(|error| malformed_item(error.to_string()))?
    }
    .with_count(count);
    let max_stack = stack
        .max_stack_size()
        .map_err(|error| malformed_item(error.to_string()))?;
    if check_overstack && count > max_stack {
        return Err(translatable(
            "arguments.item.overstacked",
            vec![
                NbtSummaryValue::String(resource_id.into()),
                NbtSummaryValue::Int(max_stack),
            ],
        ));
    }
    if !stack.is_empty()
        && bbb_server_datapack::item_stack_template::validate_strict(stack.summary().clone())
            .item_id
            .is_none()
    {
        return Err(malformed_item("the stack fails ItemStack.validateStrict"));
    }
    Ok(stack)
}

/// `ERROR_MALFORMED_ITEM` (`ItemInput.java:17-19`): `createItemStack` reports a
/// stack `validateStrict` refuses.
fn malformed_item(message: impl Into<String>) -> NbtSummaryValue {
    translatable(
        "arguments.item.malformed",
        vec![NbtSummaryValue::String(message.into().into())],
    )
}

pub(super) fn persistent_decode_context(
    resolver: &crate::world::registry_runtime::PlayerItemStackResolver,
) -> PersistentNbtDecodeContext<'_> {
    PersistentNbtDecodeContext::with_resolver(resolver)
}

/// The refusals `ItemParser.State.readComponents` (`ItemParser.java:143-191`)
/// raises, through the sentence the patch scanner built.
///
/// `CommandSyntaxErrorUtf16` carries a kind and a cursor but not the reason, so the
/// message is the channel these two layers share; the test
/// `the_item_patch_refusals_the_reader_matches_on_are_pinned` in
/// `crates/bbb-server-world/src/tests/commands/items.rs` pins every one of them
/// from this side.
fn item_patch_failure(command: &[u16], error: CommandSyntaxErrorUtf16) -> ParseFailure {
    let cursor = error.cursor;
    let message = &error.message;
    if let Some(resource_id) = message
        .units()
        .get(..23)
        .filter(|units| {
            "unknown item component "
                .encode_utf16()
                .eq(units.iter().copied())
        })
        .map(|_| &message.units()[23..])
    {
        // `ERROR_UNKNOWN_COMPONENT` (`:38-40`, thrown at `:200-206`): a name
        // the component registry does not hold and a transient component take
        // the same refusal (`!component.isTransient()`, `:201`).
        return ParseFailure {
            message: translatable(
                "arguments.item.component.unknown",
                vec![NbtSummaryValue::string(resource_id)],
            ),
            cursor: Some(cursor),
        };
    }
    if let Some(resource_id) = message
        .units()
        .get(..25)
        .filter(|units| {
            "duplicate item component "
                .encode_utf16()
                .eq(units.iter().copied())
        })
        .map(|_| &message.units()[25..])
    {
        // `ERROR_REPEATED_COMPONENT` (`:47-49`) is thrown by `create`, without
        // context (`:154-155`, `:163-165`), and `parse` rethrows it as is
        // (`:90-99`).
        return ParseFailure {
            message: translatable(
                "arguments.item.component.repeated",
                vec![NbtSummaryValue::string(resource_id)],
            ),
            cursor: None,
        };
    }
    if message == "invalid resource location" {
        // `Identifier.read`'s `ERROR_INVALID` (`Identifier.java:189-199`).
        return ParseFailure {
            message: translatable("argument.id.invalid", Vec::new()),
            cursor: Some(cursor),
        };
    }
    if message == "invalid item component SNBT" {
        // `TagParser.parseAsArgument` (`TagParser.java:79-81`). The grammar's
        // own refusal is reported by the caller; this is left for a bound this
        // repository adds, which carries no Vanilla key.
        return ParseFailure {
            message: translatable("argument.nbt.invalid", Vec::new()),
            cursor: Some(cursor),
        };
    }
    if message == "expected item component assignment" {
        // `this.reader.expect('=')` (`:169`) is Brigadier's
        // `readerExpectedSymbol`, a literal rather than a translation key
        // (`StringReader.java:266-272`, `BuiltInExceptions.java:60-62`).
        return ParseFailure {
            message: component_literal_nbt("Expected '='"),
            cursor: Some(cursor),
        };
    }
    if message == "unclosed item component patch" {
        // A `,` was consumed and the list then ended, which vanilla refuses
        // with `ERROR_EXPECTED_COMPONENT` (`:44-46`, thrown at `:184-186`);
        // otherwise `expect(']')` (`:189`) is what failed.
        let after_separator = command
            .get(..cursor)
            .unwrap_or_default()
            .iter()
            .copied()
            .rev()
            .find(|unit| !char::from_u32(u32::from(*unit)).is_some_and(char::is_whitespace))
            == Some(44);
        return ParseFailure {
            message: if after_separator {
                translatable("arguments.item.component.expected", Vec::new())
            } else {
                component_literal_nbt("Expected ']'")
            },
            cursor: Some(cursor),
        };
    }
    // `expect(']')` (`:189`) after the loop breaks on anything that is not the
    // component separator (`:177-179`).
    ParseFailure {
        message: component_literal_nbt("Expected ']'"),
        cursor: Some(cursor),
    }
}
