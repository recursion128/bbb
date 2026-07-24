"""Tree-sitter-backed vanilla block registry declaration extraction.

Java structure and expression facts come exclusively from
:mod:`vanilla_java_source`. This module only recognizes the Minecraft block
registration and ``BlockIds`` vocabulary.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Sequence

from vanilla_java_source import (
    JavaExpression,
    JavaSource,
    expression_fact,
    resolve_class_hierarchy,
)


class BlockRegistryCatalogExtractionError(ValueError):
    """An explicit unsupported or inconsistent block registry source."""


BLOCKS_OWNER = "net.minecraft.world.level.block.Blocks"
BLOCK_IDS_OWNER = "net.minecraft.references.BlockIds"
BLOCK_ENTITY_TYPES_OWNER = "net.minecraft.world.level.block.entity.BlockEntityType"
REGISTER_HELPERS = {
    "register",
    "registerBed",
    "registerLegacyStair",
    "registerStair",
    "registerStainedGlass",
}
WEATHERING_COPPER_BLOCKS_TYPE = "WeatheringCopperBlocks"
WEATHERING_COPPER_VARIANTS = (
    ("unaffected", "", ""),
    ("exposed", "exposed_", "EXPOSED_"),
    ("weathered", "weathered_", "WEATHERED_"),
    ("oxidized", "oxidized_", "OXIDIZED_"),
    ("waxed", "waxed_", "WAXED_"),
    ("waxed_exposed", "waxed_exposed_", "WAXED_EXPOSED_"),
    ("waxed_weathered", "waxed_weathered_", "WAXED_WEATHERED_"),
    ("waxed_oxidized", "waxed_oxidized_", "WAXED_OXIDIZED_"),
)
BLOCK_FACTORY_CLASSES_BY_HELPER = {
    "registerBed": "BedBlock",
    "registerLegacyStair": "StairBlock",
    "registerStair": "StairBlock",
    "registerStainedGlass": "StainedGlassBlock",
}


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise BlockRegistryCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _arguments(source: JavaSource, expression: JavaExpression) -> list[JavaExpression]:
    arguments = expression.node.child_by_field_name("arguments")
    if arguments is None:
        _fail(source, expression, "registration expression lacks arguments")
    return [source.expression(child) for child in arguments.named_children]


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


def field_constant(
    source: JavaSource, expression: JavaExpression, owner: str
) -> str | None:
    node = expression.node
    if node.type not in {"field_access", "scoped_identifier"}:
        return None
    object_node = node.child_by_field_name("object") or node.child_by_field_name("scope")
    field_node = node.child_by_field_name("field") or node.child_by_field_name("name")
    if object_node is None or field_node is None:
        _fail(source, expression, "field constant is incomplete")
    if source.expression(object_node).canonical.replace(" ", "") != owner:
        return None
    return source.text(field_node)


def _walk_expression_facts(expression: dict):
    yield expression
    object_fact = expression.get("object")
    if object_fact is not None:
        yield from _walk_expression_facts(object_fact)
    for key in ("arguments", "children"):
        for child in expression.get(key, []):
            yield from _walk_expression_facts(child)


def _factory_class_from_expression(expression: dict) -> str | None:
    for fact in _walk_expression_facts(expression):
        if fact.get("kind") == "method_reference":
            owner, separator, constructor = fact["canonical"].partition("::")
            if separator and constructor == "new":
                return owner.rsplit(".", 1)[-1]
        if fact.get("kind") == "object_creation_expression":
            return fact.get("name", "").rsplit(".", 1)[-1].split("<", 1)[0]
    return None


def block_factory_class(declaration: dict) -> str:
    initializer = declaration["initializer"]
    arguments = initializer.get("arguments", [])
    group_variant = declaration.get("group_variant")
    if group_variant is not None:
        factory_index = 2 if group_variant.startswith("waxed") else 3
        if len(arguments) <= factory_index:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: incomplete WeatheringCopperBlocks factory"
            )
        factory_class = _factory_class_from_expression(arguments[factory_index])
    else:
        helper = initializer.get("name")
        if helper in BLOCK_FACTORY_CLASSES_BY_HELPER:
            return BLOCK_FACTORY_CLASSES_BY_HELPER[helper]
        if helper != "register":
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: unsupported factory root {helper!r}"
            )
        if len(arguments) == 2:
            return "Block"
        if len(arguments) != 3:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: register has {len(arguments)} arguments"
            )
        factory_class = _factory_class_from_expression(arguments[1])
    if not factory_class:
        raise BlockRegistryCatalogExtractionError(
            f"Blocks.{declaration['field']}: block factory class is not explicit"
        )
    return factory_class


def _is_block_behavior_method(
    source: JavaSource,
    method,
    method_name: str,
    parameter_types: tuple[str, ...],
) -> bool:
    if (
        method.is_constructor
        or method.name != method_name
        or len(method.parameters) != len(parameter_types)
    ):
        return False
    return_type = method.node.child_by_field_name("type")
    actual_parameter_types = [
        parameter.node.child_by_field_name("type") for parameter in method.parameters
    ]
    return (
        return_type is not None
        and all(
            parameter_type is not None for parameter_type in actual_parameter_types
        )
        and source.text(return_type) == "InteractionResult"
        and tuple(
            source.text(parameter_type) for parameter_type in actual_parameter_types
        )
        == parameter_types
    )


def _extract_block_behavior_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    method_name: str,
    parameter_types: tuple[str, ...],
) -> dict[str, str | None]:
    types = [item for source in block_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_fqcn = {item.fqcn: item for item in types}
    by_simple_name: dict[str, list] = {}
    for item in types:
        by_simple_name.setdefault(item.simple_name, []).append(item)
    behavior_owners = {
        method.owner
        for source in block_class_sources
        for method in source.methods
        if _is_block_behavior_method(
            source, method, method_name, parameter_types
        )
    }

    owners: dict[str, str | None] = {}
    for factory_class in sorted(
        {block_factory_class(declaration) for declaration in declarations}
    ):
        matches = by_simple_name.get(factory_class, [])
        if len(matches) != 1:
            raise BlockRegistryCatalogExtractionError(
                f"expected exactly one block class {factory_class}, got "
                f"{len(matches)}"
            )
        block_type = matches[0]
        seen = set()
        while block_type.fqcn not in seen:
            seen.add(block_type.fqcn)
            if block_type.fqcn in behavior_owners:
                owners[factory_class] = (
                    None
                    if block_type.simple_name == "BlockBehaviour"
                    else block_type.simple_name
                )
                break
            if (
                block_type.superclass is None
                or block_type.superclass not in by_fqcn
            ):
                raise BlockRegistryCatalogExtractionError(
                    f"{block_type.source_path}: {factory_class} does not resolve to "
                    f"BlockBehaviour.{method_name}"
                )
            block_type = by_fqcn[block_type.superclass]
        else:
            raise BlockRegistryCatalogExtractionError(
                f"superclass cycle while resolving {factory_class}.{method_name}"
            )
    return owners


def extract_block_use_item_on_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    return _extract_block_behavior_override_owners(
        declarations,
        block_class_sources,
        "useItemOn",
        (
            "ItemStack",
            "BlockState",
            "Level",
            "BlockPos",
            "Player",
            "InteractionHand",
            "BlockHitResult",
        ),
    )


def extract_block_use_without_item_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    return _extract_block_behavior_override_owners(
        declarations,
        block_class_sources,
        "useWithoutItem",
        ("BlockState", "Level", "BlockPos", "Player", "BlockHitResult"),
    )


def resolve_block_resource_path(
    source: JavaSource,
    expression: JavaExpression,
    block_id_constants: dict[str, str],
) -> str:
    if expression.node.type == "string_literal":
        return _string(source, expression)
    constant = field_constant(source, expression, "BlockIds")
    if constant is None:
        _fail(source, expression, "unsupported block resource id expression")
    resource_path = block_id_constants.get(constant)
    if resource_path is None:
        _fail(source, expression, f"unresolved BlockIds.{constant}")
    return resource_path


def extract_block_id_constants(source: JavaSource) -> dict[str, str]:
    constants: dict[str, str] = {}
    for field in source.fields_for(BLOCK_IDS_OWNER):
        if field.type_spelling.replace(" ", "") != "ResourceKey<Block>":
            continue
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or source.expression(field.node)
            _fail(source, expression, f"BlockIds.{field.name} lacks createKey initializer")
        name = initializer.node.child_by_field_name("name")
        arguments = _arguments(source, initializer)
        if name is None or source.text(name) != "createKey" or len(arguments) != 1:
            _fail(source, initializer, f"BlockIds.{field.name} is not createKey(string)")
        if field.name in constants:
            _fail(source, initializer, f"duplicate BlockIds.{field.name}")
        constants[field.name] = _string(source, arguments[0])
    if not constants:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: no BlockIds createKey declarations found"
        )
    return constants


def extract_block_entity_block_constants(source: JavaSource) -> dict[str, str]:
    constants: dict[str, str] = {}
    for field in source.fields_for(BLOCK_ENTITY_TYPES_OWNER):
        if not field.type_spelling.replace(" ", "").startswith("BlockEntityType<"):
            continue
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or source.expression(field.node)
            _fail(source, expression, f"BlockEntityType.{field.name} lacks register call")
        name = initializer.node.child_by_field_name("name")
        arguments = _arguments(source, initializer)
        if name is None or source.text(name) != "register" or len(arguments) < 3:
            _fail(
                source,
                initializer,
                f"BlockEntityType.{field.name} is not register(name, factory, blocks...)",
            )
        for block in arguments[2:]:
            constant = field_constant(source, block, "Blocks")
            if constant is None:
                _fail(source, block, "block entity valid block is not a Blocks constant")
            if constant in constants:
                _fail(source, block, f"duplicate block entity valid block Blocks.{constant}")
            constants[constant] = field.name
    if not constants:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: no block entity valid blocks found"
        )
    return constants


@dataclass(frozen=True)
class ExtractedBlockRegistryCatalog:
    declarations: tuple[dict[str, Any], ...]

    def as_json_value(self) -> dict[str, Any]:
        return {
            "vanilla_version": "26.1",
            "generated_by": "scripts/generate-block-registry-catalog.py",
            "update_command": (
                "scripts/vanilla-source-tools.sh "
                "scripts/generate-block-registry-catalog.py"
            ),
            "check_command": (
                "scripts/vanilla-source-tools.sh "
                "scripts/generate-block-registry-catalog.py --check"
            ),
            "sources": [
                "net/minecraft/references/BlockIds.java",
                "net/minecraft/world/level/block/Blocks.java",
                "net/minecraft/world/level/block/entity/BlockEntityType.java",
            ],
            "declarations": list(self.declarations),
        }


def extract_block_registry_catalog(
    blocks_source: JavaSource,
    block_ids_source: JavaSource,
    block_entity_types_source: JavaSource,
) -> ExtractedBlockRegistryCatalog:
    constants = extract_block_id_constants(block_ids_source)
    block_entity_fields = extract_block_entity_block_constants(block_entity_types_source)
    fields = [
        field
        for field in blocks_source.fields_for(BLOCKS_OWNER)
        if field.type_spelling in {"Block", WEATHERING_COPPER_BLOCKS_TYPE}
    ]
    declarations: list[dict[str, Any]] = []
    for field in fields:
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or blocks_source.expression(field.node)
            _fail(blocks_source, expression, f"Blocks.{field.name} lacks registration call")
        name_node = initializer.node.child_by_field_name("name")
        if name_node is None:
            _fail(blocks_source, initializer, "block registration lacks helper name")
        helper = blocks_source.text(name_node)
        if field.type_spelling == WEATHERING_COPPER_BLOCKS_TYPE:
            arguments = _arguments(blocks_source, initializer)
            object_node = initializer.node.child_by_field_name("object")
            object_name = (
                blocks_source.expression(object_node).canonical.replace(" ", "")
                if object_node is not None
                else ""
            )
            if helper != "create" or object_name != WEATHERING_COPPER_BLOCKS_TYPE:
                _fail(
                    blocks_source,
                    initializer,
                    "WeatheringCopperBlocks field is not initialized by WeatheringCopperBlocks.create",
                )
            if len(arguments) != 5:
                _fail(
                    blocks_source,
                    initializer,
                    "WeatheringCopperBlocks.create does not have five arguments",
                )
            base_resource_path = _string(blocks_source, arguments[0])
            for variant, resource_prefix, field_prefix in WEATHERING_COPPER_VARIANTS:
                declarations.append(
                    {
                        "field": f"{field_prefix}{field.name}",
                        "declaration_order": len(declarations),
                        "resource_id": f"minecraft:{resource_prefix}{base_resource_path}",
                        "source_path": blocks_source.relative_path,
                        "start_byte": initializer.span.start_byte,
                        "end_byte": initializer.span.end_byte,
                        "initializer": expression_fact(blocks_source, initializer),
                        "group_variant": variant,
                        "has_block_entity": f"{field_prefix}{field.name}"
                        in block_entity_fields,
                        "block_entity_type": block_entity_fields.get(
                            f"{field_prefix}{field.name}"
                        ),
                    }
                )
            continue
        if helper not in REGISTER_HELPERS:
            _fail(blocks_source, initializer, f"unsupported block register helper {helper}")
        arguments = _arguments(blocks_source, initializer)
        if not arguments:
            _fail(blocks_source, initializer, "block registration lacks resource id")
        resource_path = resolve_block_resource_path(
            blocks_source, arguments[0], constants
        )
        declarations.append(
            {
                "field": field.name,
                "declaration_order": len(declarations),
                "resource_id": f"minecraft:{resource_path}",
                "source_path": blocks_source.relative_path,
                "start_byte": initializer.span.start_byte,
                "end_byte": initializer.span.end_byte,
                "initializer": expression_fact(blocks_source, initializer),
                "has_block_entity": field.name in block_entity_fields,
                "block_entity_type": block_entity_fields.get(field.name),
            }
        )
    expected_declarations = sum(
        len(WEATHERING_COPPER_VARIANTS)
        if field.type_spelling == WEATHERING_COPPER_BLOCKS_TYPE
        else 1
        for field in fields
    )
    if not declarations or len(declarations) != expected_declarations:
        raise BlockRegistryCatalogExtractionError(
            f"{blocks_source.relative_path}: failed to close all block declarations"
        )
    return ExtractedBlockRegistryCatalog(tuple(declarations))
