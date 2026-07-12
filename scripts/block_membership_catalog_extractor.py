"""Tree-sitter-backed extraction for pure vanilla block memberships."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaSource, descendants
from vanilla_tag_source import VanillaTagSourceError, resolve_tag_values


class BlockMembershipCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class BlockMembershipEntry:
    constant_name: str
    resource_id: str


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise BlockMembershipCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected string literal block resource path")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "expected non-empty block resource path")
    return value


def _builder_property_blocks(
    source: JavaSource, property_name: str, expected_count: int
) -> list[BlockMembershipEntry]:
    entries: list[BlockMembershipEntry] = []
    for field in source.fields_for("net.minecraft.world.level.block.Blocks"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            continue
        calls = [
            call
            for call in source.calls
            if initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= initializer.span.end_byte
        ]
        if not any(call.name == property_name for call in calls):
            continue
        register_calls = [call for call in calls if call.name == "register"]
        if len(register_calls) != 1 or not register_calls[0].arguments:
            _fail(source, initializer, f"unsupported {property_name} block registration")
        entries.append(
            BlockMembershipEntry(
                field.name,
                "minecraft:" + _string(source, register_calls[0].arguments[0]),
            )
        )

    if len(entries) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} {property_name} blocks, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate {property_name} block constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate {property_name} block resource id"
        )
    return entries


def no_terrain_particle_blocks(
    source: JavaSource, expected_count: int = 2
) -> list[BlockMembershipEntry]:
    return _builder_property_blocks(source, "noTerrainParticles", expected_count)


def air_blocks(
    source: JavaSource, expected_count: int = 3
) -> list[BlockMembershipEntry]:
    return _builder_property_blocks(source, "air", expected_count)


def _constructed_type_name(source: JavaSource, node) -> str | None:
    if node.type == "object_creation_expression":
        type_node = node.child_by_field_name("type")
        if type_node is None:
            _fail(source, source.expression(node), "object creation lacks type")
        return source.expression(type_node).canonical.replace(" ", "")
    if node.type == "method_reference" and any(
        child.type == "new" for child in node.children
    ):
        type_nodes = [
            child for child in node.named_children if child.type != "type_arguments"
        ]
        if len(type_nodes) != 1:
            _fail(source, source.expression(node), "constructor reference lacks unique type")
        return source.expression(type_nodes[0]).canonical.replace(" ", "")
    return None


def constructor_family_blocks(
    source: JavaSource,
    constructor_types: frozenset[str],
    expected_count: int,
) -> list[BlockMembershipEntry]:
    if not constructor_types:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: constructor family cannot be empty"
        )
    entries: list[BlockMembershipEntry] = []
    for field in source.fields_for("net.minecraft.world.level.block.Blocks"):
        if field.type_spelling != "Block" or field.initializer is None:
            continue
        constructed_types = {
            type_name
            for node in descendants(field.initializer.node)
            if (type_name := _constructed_type_name(source, node)) in constructor_types
        }
        if not constructed_types:
            continue
        if len(constructed_types) != 1:
            _fail(
                source,
                field.initializer,
                "block registration matches multiple constructor family types",
            )
        register_calls = [
            call
            for call in source.calls
            if field.initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= field.initializer.span.end_byte
            and call.name == "register"
        ]
        if len(register_calls) != 1 or not register_calls[0].arguments:
            _fail(source, field.initializer, "constructor family lacks unique register call")
        entries.append(
            BlockMembershipEntry(
                field.name,
                "minecraft:" + _string(source, register_calls[0].arguments[0]),
            )
        )

    family = ", ".join(sorted(constructor_types))
    if len(entries) != expected_count:
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} blocks constructed by "
            f"{{{family}}}, extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate constructor-family block constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BlockMembershipCatalogExtractionError(
            f"{source.relative_path}: duplicate constructor-family block resource id"
        )
    return entries


def block_tag_membership(
    tag_resources: dict[str, tuple[str, bytes]],
    tag_id: str,
    expected_count: int,
) -> tuple[str, ...]:
    try:
        return resolve_tag_values(
            tag_resources, tag_id, expected_count, registry_name="block"
        )
    except VanillaTagSourceError as error:
        raise BlockMembershipCatalogExtractionError(str(error)) from error
