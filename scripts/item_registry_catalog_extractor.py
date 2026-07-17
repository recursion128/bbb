"""Tree-sitter-backed vanilla item registry declaration extraction.

This module owns only the Minecraft item registration vocabulary. Java lexing,
declaration discovery, delimiter handling, and malformed-CST diagnostics remain
owned by :mod:`vanilla_java_source`.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any

from vanilla_java_source import JavaExpression, JavaSource, expression_fact


class ItemRegistryCatalogExtractionError(ValueError):
    pass


ITEMS_OWNER = "net.minecraft.world.item.Items"
ITEM_OWNER = "net.minecraft.world.item.Item"
ITEM_IDS_OWNER = "net.minecraft.references.ItemIds"
ITEM_FIELD_TYPES = {"Item", "WeatheringCopperItems"}


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise ItemRegistryCatalogExtractionError(
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


def _integer(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type != "decimal_integer_literal":
        _fail(source, expression, "expected a Java decimal integer literal")
    try:
        return int(expression.canonical.replace("_", ""), 10)
    except ValueError:
        _fail(source, expression, "invalid Java integer literal")


def _arguments(source: JavaSource, node) -> list[JavaExpression]:
    arguments = node.child_by_field_name("arguments")
    if arguments is None:
        raise ItemRegistryCatalogExtractionError(
            f"{source.relative_path}: expression lacks an argument list"
        )
    return [
        source.expression(child)
        for child in arguments.named_children
        if child.type not in {"line_comment", "block_comment"}
    ]


@dataclass(frozen=True)
class ExtractedItemRegistryCatalog:
    item_id_constants: dict[str, str]
    item_constants: dict[str, int]
    declarations: tuple[dict[str, Any], ...]

    def as_json_value(self) -> dict[str, Any]:
        return {
            "vanilla_version": "26.1",
            "generated_by": "scripts/generate-item-registry-catalog.py",
            "update_command": "scripts/vanilla-source-tools.sh scripts/generate-item-registry-catalog.py",
            "check_command": "scripts/vanilla-source-tools.sh scripts/generate-item-registry-catalog.py --check",
            "sources": [
                "net/minecraft/references/ItemIds.java",
                "net/minecraft/world/item/Items.java",
            ],
            "item_id_constants": self.item_id_constants,
            "declarations": list(self.declarations),
        }


def extract_item_registry_catalog(
    items_source: JavaSource, item_ids_source: JavaSource, item_source: JavaSource
) -> ExtractedItemRegistryCatalog:
    constants: dict[str, str] = {}
    for field in item_ids_source.fields_for(ITEM_IDS_OWNER):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            raise ItemRegistryCatalogExtractionError(
                f"{item_ids_source.relative_path}: ItemIds.{field.name} lacks createKey initializer"
            )
        name = initializer.node.child_by_field_name("name")
        arguments = _arguments(item_ids_source, initializer.node)
        if name is None or item_ids_source.text(name) != "createKey" or len(arguments) != 1:
            _fail(item_ids_source, initializer, "expected createKey(string) initializer")
        constants[field.name] = _string(item_ids_source, arguments[0])

    item_constant_names = {"DEFAULT_MAX_STACK_SIZE", "ABSOLUTE_MAX_STACK_SIZE"}
    item_constants = {}
    for field in item_source.fields_for(ITEM_OWNER):
        if field.name not in item_constant_names:
            continue
        if field.type_spelling != "int" or field.initializer is None:
            raise ItemRegistryCatalogExtractionError(
                f"{item_source.relative_path}: Item.{field.name} must be an initialized int"
            )
        if not {"public", "static", "final"}.issubset(field.modifiers):
            raise ItemRegistryCatalogExtractionError(
                f"{item_source.relative_path}: Item.{field.name} must be public static final"
            )
        item_constants[field.name] = _integer(item_source, field.initializer)
    if set(item_constants) != item_constant_names:
        raise ItemRegistryCatalogExtractionError(
            f"{item_source.relative_path}: expected item constants {sorted(item_constant_names)}, "
            f"extracted {sorted(item_constants)}"
        )
    if not 0 < item_constants["DEFAULT_MAX_STACK_SIZE"] <= item_constants["ABSOLUTE_MAX_STACK_SIZE"]:
        raise ItemRegistryCatalogExtractionError(
            f"{item_source.relative_path}: invalid item max stack size constants"
        )

    declarations: list[dict[str, Any]] = []
    fields = [
        field
        for field in items_source.fields_for(ITEMS_OWNER)
        if field.type_spelling in ITEM_FIELD_TYPES
    ]
    for field in fields:
        if field.initializer is None:
            raise ItemRegistryCatalogExtractionError(
                f"{items_source.relative_path}: Items.{field.name} lacks initializer"
            )
        if field.initializer.node.type != "method_invocation":
            _fail(items_source, field.initializer, "item initializer must be a method invocation")
        declarations.append(
            {
                "field": field.name,
                "field_type": field.type_spelling,
                "declaration_order": field.declaration_order,
                "source_path": items_source.relative_path,
                "start_byte": field.initializer.span.start_byte,
                "end_byte": field.initializer.span.end_byte,
                "initializer": expression_fact(items_source, field.initializer),
            }
        )

    if len(declarations) != len(fields) or not declarations:
        raise ItemRegistryCatalogExtractionError(
            f"{items_source.relative_path}: failed to close all item declarations"
        )
    return ExtractedItemRegistryCatalog(constants, item_constants, tuple(declarations))
