"""Tree-sitter-backed extraction for command argument and selector catalogs."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import (
    JavaExpression,
    JavaSource,
    descendants,
    type_body_members,
)


class CommandArgumentCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class CommandArgumentEntry:
    resource_id: str


@dataclass(frozen=True)
class CommandSlotEntry:
    name: str
    single: bool


@dataclass(frozen=True)
class CommandEntitySelectorOptionEntry:
    name: str
    kind: str


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise CommandArgumentCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string(source: JavaSource, expression: JavaExpression, description: str) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, f"{description} is not a string literal")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid {description} literal: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, f"{description} is empty or non-string")
    return value


def command_argument_entries(
    source: JavaSource, expected_count: int = 57
) -> list[CommandArgumentEntry]:
    methods = [method for method in source.methods if method.name == "bootstrap"]
    if len(methods) != 1 or methods[0].body is None:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected exactly one bootstrap method with a body"
        )
    method = methods[0]
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[CommandArgumentEntry] = []
    for call in calls:
        if len(call.arguments) != 4 or call.arguments[0].canonical != "registry":
            _fail(source, source.expression(call.node), "unsupported argument register call")
        resource = call.arguments[1]
        value = _string(source, resource, "argument id")
        entries.append(CommandArgumentEntry(value if ":" in value else f"minecraft:{value}"))
    if len(entries) != expected_count:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} argument registrations, "
            f"extracted {len(entries)}"
        )
    resources = [entry.resource_id for entry in entries]
    if len(resources) != len(set(resources)):
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: duplicate command argument resource id"
        )
    return entries


def _entity_selector_option_kind(
    source: JavaSource, predicate: JavaExpression
) -> str:
    if predicate.node.type != "lambda_expression":
        _fail(source, predicate, "entity selector canUse predicate is not a lambda")
    parameter = predicate.node.child_by_field_name("parameters")
    body = predicate.node.child_by_field_name("body")
    if parameter is None or parameter.type != "identifier" or body is None:
        _fail(source, predicate, "unsupported entity selector canUse lambda shape")
    owner = source.text(parameter)
    canonical = source.expression(body).canonical
    kind_by_predicate = {
        f"! {owner}.hasNameEquals ()": "Name",
        f"{owner}.getDistance () == null": "Once",
        f"{owner}.getLevel () == null": "Once",
        f"{owner}.getX () == null": "Once",
        f"{owner}.getY () == null": "Once",
        f"{owner}.getZ () == null": "Once",
        f"{owner}.getDeltaX () == null": "Once",
        f"{owner}.getDeltaY () == null": "Once",
        f"{owner}.getDeltaZ () == null": "Once",
        f"{owner}.getRotX () == null": "Once",
        f"{owner}.getRotY () == null": "Once",
        f"! {owner}.isCurrentEntity () && ! {owner}.isLimited ()": "Limit",
        f"! {owner}.isCurrentEntity () && ! {owner}.isSorted ()": "Sort",
        f"! {owner}.hasGamemodeEquals ()": "Gamemode",
        f"! {owner}.hasTeamEquals ()": "Team",
        f"! {owner}.isTypeLimited ()": "Type",
        "true": "Repeatable",
        f"! {owner}.hasScores ()": "Once",
        f"! {owner}.hasAdvancements ()": "Once",
    }
    kind = kind_by_predicate.get(canonical)
    if kind is None:
        _fail(
            source,
            source.expression(body),
            f"unsupported entity selector canUse predicate {canonical!r}",
        )
    return kind


def command_entity_selector_option_entries(
    source: JavaSource, expected_count: int = 21
) -> list[CommandEntitySelectorOptionEntry]:
    methods = [method for method in source.methods if method.name == "bootStrap"]
    if len(methods) != 1 or methods[0].body is None:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected exactly one bootStrap method with a body"
        )
    method = methods[0]
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))

    entries: list[CommandEntitySelectorOptionEntry] = []
    for call in calls:
        expression = source.expression(call.node)
        if len(call.arguments) != 4:
            _fail(source, expression, "unsupported entity selector register call")
        entries.append(
            CommandEntitySelectorOptionEntry(
                name=_string(source, call.arguments[0], "entity selector option name"),
                kind=_entity_selector_option_kind(source, call.arguments[2]),
            )
        )

    if len(entries) != expected_count:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} entity selector options, "
            f"extracted {len(entries)}"
        )
    names = [entry.name for entry in entries]
    if len(names) != len(set(names)):
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: duplicate entity selector option name"
        )
    return entries


def command_slot_entries(
    source: JavaSource, expected_count: int = 165
) -> list[CommandSlotEntry]:
    fields = [field for field in source.fields if field.name == "SLOTS"]
    if (
        len(fields) != 1
        or fields[0].initializer is None
        or fields[0].type_spelling.replace(" ", "") != "List<SlotRange>"
    ):
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected one initialized List<SlotRange> SLOTS field"
        )
    initializer = fields[0].initializer
    lambdas = [
        node for node in descendants(initializer.node) if node.type == "lambda_expression"
    ]
    if len(lambdas) != 1:
        _fail(source, initializer, "SLOTS initializer must contain exactly one lambda")
    parameter = lambdas[0].child_by_field_name("parameters")
    body = lambdas[0].child_by_field_name("body")
    if parameter is None or parameter.type != "identifier" or body is None:
        _fail(source, source.expression(lambdas[0]), "unsupported SLOTS lambda shape")
    output_name = source.text(parameter)
    calls = [
        call
        for call in source.calls
        if call.qualifier is None
        and body.start_byte <= call.span.start_byte
        and call.span.end_byte <= body.end_byte
        and call.arguments
        and call.arguments[0].canonical == output_name
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))

    entries: list[CommandSlotEntry] = []
    for call in calls:
        expression = source.expression(call.node)
        if call.name == "addSingleSlot":
            if len(call.arguments) != 3:
                _fail(source, expression, "addSingleSlot argument shape changed")
            entries.append(
                CommandSlotEntry(_string(source, call.arguments[1], "slot name"), True)
            )
        elif call.name == "addSlots":
            if len(call.arguments) < 3:
                _fail(source, expression, "addSlots argument shape changed")
            entries.append(
                CommandSlotEntry(_string(source, call.arguments[1], "slot name"), False)
            )
        elif call.name == "addSlotRange":
            if len(call.arguments) != 4:
                _fail(source, expression, "addSlotRange argument shape changed")
            prefix = _string(source, call.arguments[1], "slot range prefix")
            size_expression = call.arguments[3]
            if size_expression.node.type != "decimal_integer_literal":
                _fail(source, size_expression, "slot range size is not a decimal literal")
            try:
                size = int(source.text(size_expression.node).replace("_", ""))
            except ValueError as error:
                _fail(source, size_expression, f"invalid slot range size: {error}")
            if size <= 0:
                _fail(source, size_expression, "slot range size must be positive")
            entries.extend(
                CommandSlotEntry(f"{prefix}{index}", True) for index in range(size)
            )
            entries.append(CommandSlotEntry(f"{prefix}*", False))
        else:
            _fail(source, expression, f"unsupported SLOTS helper {call.name}")

    if len(entries) != expected_count:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} command slots, "
            f"extracted {len(entries)}"
        )
    names = [entry.name for entry in entries]
    if len(names) != len(set(names)):
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: duplicate command slot name"
        )
    return entries


def serialized_enum_names(
    source: JavaSource, expected_count: int
) -> list[str]:
    enums = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.node.parent.type == "program"
    ]
    if len(enums) != 1 or not any(
        spelling.replace(" ", "").endswith("StringRepresentable")
        for spelling in enums[0].interface_spellings
    ):
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected one top-level StringRepresentable enum"
        )

    names = []
    for node in type_body_members(enums[0].body_node):
        if node.type != "enum_constant":
            continue
        arguments = node.child_by_field_name("arguments")
        if arguments is None:
            _fail(source, source.expression(node), "enum constant lacks arguments")
        string_arguments = [
            source.expression(child)
            for child in arguments.named_children
            if child.type == "string_literal"
        ]
        if len(string_arguments) != 1:
            _fail(
                source,
                source.expression(node),
                "enum constant must have exactly one serialized-name literal",
            )
        names.append(_string(source, string_arguments[0], "serialized enum name"))

    if len(names) != expected_count:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} serialized enum names, "
            f"extracted {len(names)}"
        )
    if len(names) != len(set(names)):
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: duplicate serialized enum name"
        )
    return names
