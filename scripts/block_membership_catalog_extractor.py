"""Tree-sitter-backed extraction for pure vanilla block memberships."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaSource


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


def block_tag_membership(
    tag_resources: dict[str, tuple[str, bytes]],
    tag_id: str,
    expected_count: int,
) -> tuple[str, ...]:
    parsed: dict[str, list[str]] = {}
    for resource_id, (relative_path, raw) in sorted(tag_resources.items()):
        try:
            root = json.loads(raw)
        except (json.JSONDecodeError, UnicodeDecodeError) as error:
            raise BlockMembershipCatalogExtractionError(
                f"{relative_path}: invalid block tag JSON: {error}"
            ) from error
        if (
            not isinstance(root, dict)
            or set(root) - {"replace", "values"}
            or root.get("replace", False) not in {True, False}
        ):
            raise BlockMembershipCatalogExtractionError(
                f"{relative_path}: unsupported block tag root"
            )
        values = root.get("values")
        if not isinstance(values, list):
            raise BlockMembershipCatalogExtractionError(
                f"{relative_path}: block tag values is not an array"
            )
        parsed_values: list[str] = []
        for index, value in enumerate(values):
            if not isinstance(value, str) or not value.removeprefix("#").startswith(
                "minecraft:"
            ):
                raise BlockMembershipCatalogExtractionError(
                    f"{relative_path}: unsupported block tag value at index {index}"
                )
            parsed_values.append(value)
        parsed[resource_id] = parsed_values

    resolving: list[str] = []

    def resolve(resource_id: str) -> list[str]:
        if resource_id in resolving:
            raise BlockMembershipCatalogExtractionError(
                f"block tag reference cycle: {' -> '.join([*resolving, resource_id])}"
            )
        values = parsed.get(resource_id)
        if values is None:
            raise BlockMembershipCatalogExtractionError(
                f"missing required block tag reference {resource_id}"
            )
        resolving.append(resource_id)
        resolved: list[str] = []
        for value in values:
            resolved.extend(resolve(value[1:]) if value.startswith("#") else [value])
        resolving.pop()
        return resolved

    entries = tuple(dict.fromkeys(resolve(tag_id)))
    if len(entries) != expected_count:
        relative_path = tag_resources.get(tag_id, (tag_id, b""))[0]
        raise BlockMembershipCatalogExtractionError(
            f"{relative_path}: expected {expected_count} unique resolved block tag values, "
            f"extracted {len(entries)}"
        )
    return entries
