"""Tree-sitter-backed vanilla furnace-fuel declaration extraction.

Java lexing, declaration discovery, call structure, and malformed-CST
diagnostics remain owned by :mod:`vanilla_java_source`. This module only
recognizes the Minecraft ``FuelValues`` builder and ``ItemTags`` vocabulary.
"""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import JavaCall, JavaExpression, JavaSource


class FurnaceFuelCatalogExtractionError(ValueError):
    """An explicit unsupported or inconsistent furnace-fuel source."""


FUEL_VALUES_OWNER = "net.minecraft.world.level.block.entity.FuelValues"
ITEM_TAGS_OWNER = "net.minecraft.tags.ItemTags"
ENTRY_OWNERS = {"Items": "item", "Blocks": "block", "ItemTags": "tag"}


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise FurnaceFuelCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected a Java string literal")
    try:
        value = json.loads(source.text(expression.node))
    except json.JSONDecodeError as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "expected a non-empty Java string literal")
    return value


def _qualified_member(
    source: JavaSource, expression: JavaExpression
) -> tuple[str, str]:
    node = expression.node
    if node.type not in {"field_access", "scoped_identifier"}:
        _fail(source, expression, "fuel entry is not a qualified constant")
    object_node = node.child_by_field_name("object") or node.child_by_field_name("scope")
    field_node = node.child_by_field_name("field") or node.child_by_field_name("name")
    if object_node is None or field_node is None:
        _fail(source, expression, "fuel entry constant is incomplete")
    return (
        source.expression(object_node).canonical.replace(" ", ""),
        source.text(field_node),
    )


def extract_item_tag_constants(source: JavaSource) -> dict[str, str]:
    constants: dict[str, str] = {}
    for field in source.fields_for(ITEM_TAGS_OWNER):
        if field.type_spelling.replace(" ", "") != "TagKey<Item>":
            continue
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or source.expression(field.node)
            _fail(source, expression, f"ItemTags.{field.name} lacks bind initializer")
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if (
            name is None
            or source.text(name) != "bind"
            or arguments is None
            or len(arguments.named_children) != 1
        ):
            _fail(source, initializer, f"ItemTags.{field.name} is not bind(string)")
        resource_path = _string(
            source, source.expression(arguments.named_children[0])
        )
        if field.name in constants:
            _fail(source, initializer, f"duplicate ItemTags.{field.name}")
        constants[field.name] = f"minecraft:{resource_path}"
    if not constants:
        raise FurnaceFuelCatalogExtractionError(
            f"{source.relative_path}: no ItemTags bind declarations found"
        )
    return constants


@dataclass(frozen=True)
class FurnaceFuelOperation:
    action: str
    entry_kind: str
    constant: str
    resource_id: str
    source_path: str
    start_byte: int
    end_byte: int

    def as_json_value(self) -> dict[str, str | int]:
        return {
            "action": self.action,
            "entry_kind": self.entry_kind,
            "constant": self.constant,
            "resource_id": self.resource_id,
            "source_path": self.source_path,
            "start_byte": self.start_byte,
            "end_byte": self.end_byte,
        }


def _builder_method_calls(source: JavaSource) -> list[JavaCall]:
    candidates: list[list[JavaCall]] = []
    for method in source.methods_for(FUEL_VALUES_OWNER):
        if method.name != "vanillaBurnTimes" or method.body is None:
            continue
        calls = [
            call
            for call in source.calls_for(FUEL_VALUES_OWNER)
            if method.body.start_byte <= call.span.start_byte
            and call.span.end_byte <= method.body.end_byte
            and call.name in {"add", "remove"}
        ]
        if calls:
            calls.sort(key=lambda call: (call.span.end_byte, call.span.start_byte))
            candidates.append(calls)
    if len(candidates) != 1:
        raise FurnaceFuelCatalogExtractionError(
            f"{source.relative_path}: expected one vanillaBurnTimes builder, "
            f"found {len(candidates)}"
        )
    return candidates[0]


def _is_fuel_builder_chain(source: JavaSource, expression: JavaExpression) -> bool:
    node = expression.node
    if node.type == "object_creation_expression":
        type_node = node.child_by_field_name("type")
        return (
            type_node is not None
            and source.expression(type_node).canonical.replace(" ", "")
            == "FuelValues.Builder"
        )
    if node.type != "method_invocation":
        return False
    name = node.child_by_field_name("name")
    object_node = node.child_by_field_name("object")
    return (
        name is not None
        and source.text(name) in {"add", "remove"}
        and object_node is not None
        and _is_fuel_builder_chain(source, source.expression(object_node))
    )


def extract_furnace_fuel_operations(
    source: JavaSource, item_tag_constants: dict[str, str]
) -> tuple[FurnaceFuelOperation, ...]:
    operations: list[FurnaceFuelOperation] = []
    for call in _builder_method_calls(source):
        if call.qualifier is None or not _is_fuel_builder_chain(source, call.qualifier):
            _fail(
                source,
                source.expression(call.node),
                f"FuelValues.Builder.{call.name} is not rooted at a FuelValues.Builder",
            )
        expected_arguments = 2 if call.name == "add" else 1
        evidence = source.expression(call.node)
        if len(call.arguments) != expected_arguments:
            _fail(
                source,
                evidence,
                f"FuelValues.Builder.{call.name} has {len(call.arguments)} arguments",
            )
        owner, constant = _qualified_member(source, call.arguments[0])
        entry_kind = ENTRY_OWNERS.get(owner)
        if entry_kind is None:
            _fail(source, call.arguments[0], f"unsupported fuel entry owner {owner}")
        if entry_kind == "tag":
            resource_id = item_tag_constants.get(constant)
            if resource_id is None:
                _fail(source, call.arguments[0], f"unresolved ItemTags.{constant}")
        else:
            resource_id = f"minecraft:{constant.lower()}"
        operations.append(
            FurnaceFuelOperation(
                call.name,
                entry_kind,
                constant,
                resource_id,
                source.relative_path,
                call.arguments[0].span.start_byte,
                call.arguments[0].span.end_byte,
            )
        )
    if not operations:
        raise FurnaceFuelCatalogExtractionError(
            f"{source.relative_path}: no furnace-fuel builder operations found"
        )
    return tuple(operations)
