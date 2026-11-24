"""Tree-sitter-backed vanilla block registry declaration extraction.

Java structure and expression facts come exclusively from
:mod:`vanilla_java_source`. This module only recognizes the Minecraft block
registration and ``BlockIds`` vocabulary.
"""

from __future__ import annotations

import json
import re
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Any

from vanilla_java_source import (
    COMMENT_NODE_KINDS,
    JavaExpression,
    JavaSource,
    JavaType,
    descendants,
    expression_fact,
    resolve_class_hierarchy,
    type_body_members,
)


class BlockRegistryCatalogExtractionError(ValueError):
    """An explicit unsupported or inconsistent block registry source."""


BLOCKS_OWNER = "net.minecraft.world.level.block.Blocks"
BLOCK_IDS_OWNER = "net.minecraft.references.BlockIds"
BLOCK_ENTITY_TYPES_OWNER = "net.minecraft.world.level.block.entity.BlockEntityType"
BLOCK_TAGS_OWNER = "net.minecraft.tags.BlockTags"
BLOCK_TAG_TYPE_SPELLING = "TagKey<Block>"
MAY_PLACE_ON_PARAMETER_TYPES = ("BlockState", "BlockGetter", "BlockPos")
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
FIRE_BLOCK_PATH = "net/minecraft/world/level/block/FireBlock.java"
FIRE_BLOCK_FQCN = "net.minecraft.world.level.block.FireBlock"
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


def _is_block_method(
    source: JavaSource,
    method,
    method_name: str,
    return_type_spelling: str,
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
        and source.text(return_type) == return_type_spelling
        and tuple(
            source.text(parameter_type) for parameter_type in actual_parameter_types
        )
        == parameter_types
    )


def _extract_block_method_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    method_name: str,
    return_type_spelling: str,
    parameter_types: tuple[str, ...],
    default_owner_fqcn: str | None,
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
        if _is_block_method(
            source,
            method,
            method_name,
            return_type_spelling,
            parameter_types,
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
                    if block_type.fqcn == default_owner_fqcn
                    else block_type.simple_name
                )
                break
            if (
                block_type.superclass is None
                or block_type.superclass not in by_fqcn
            ):
                if default_owner_fqcn is None:
                    owners[factory_class] = None
                    break
                raise BlockRegistryCatalogExtractionError(
                    f"{block_type.source_path}: {factory_class} does not resolve to "
                    f"{default_owner_fqcn}.{method_name}"
                )
            block_type = by_fqcn[block_type.superclass]
        else:
            raise BlockRegistryCatalogExtractionError(
                f"superclass cycle while resolving {factory_class}.{method_name}"
            )
    return owners


def _extract_block_interface_method_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    interface_fqcn: str,
    method_name: str,
    return_type_spelling: str,
    parameter_types: tuple[str, ...],
) -> dict[str, str | None]:
    """Resolve the concrete/default owner of one block interface method."""

    types = [item for source in block_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_fqcn = {item.fqcn: item for item in types}
    by_simple_name: dict[str, list] = {}
    for item in types:
        by_simple_name.setdefault(item.simple_name, []).append(item)
    if interface_fqcn not in by_fqcn:
        raise BlockRegistryCatalogExtractionError(
            f"unresolved block interface {interface_fqcn}"
        )
    behavior_owners = {
        method.owner
        for source in block_class_sources
        for method in source.methods
        if _is_block_method(
            source,
            method,
            method_name,
            return_type_spelling,
            parameter_types,
        )
    }

    def class_chain(block_type):
        chain = []
        seen = set()
        while block_type.fqcn not in seen:
            seen.add(block_type.fqcn)
            chain.append(block_type)
            if (
                block_type.superclass is None
                or block_type.superclass not in by_fqcn
            ):
                return chain
            block_type = by_fqcn[block_type.superclass]
        raise BlockRegistryCatalogExtractionError(
            f"superclass cycle while resolving {chain[0].simple_name}.{method_name}"
        )

    def reachable_interfaces(chain):
        pending = [
            interface
            for block_type in chain
            for interface in block_type.interfaces
        ]
        seen = set()
        ordered = []
        while pending:
            interface = pending.pop(0)
            if interface in seen:
                continue
            seen.add(interface)
            interface_type = by_fqcn.get(interface)
            if interface_type is None:
                continue
            ordered.append(interface_type)
            pending.extend(interface_type.interfaces)
        return ordered

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
        chain = class_chain(matches[0])
        interfaces = reachable_interfaces(chain)
        if not any(interface.fqcn == interface_fqcn for interface in interfaces):
            owners[factory_class] = None
            continue
        class_owner = next(
            (
                block_type
                for block_type in chain
                if block_type.fqcn in behavior_owners
            ),
            None,
        )
        if class_owner is not None:
            owners[factory_class] = class_owner.simple_name
            continue
        interface_owner = next(
            (
                interface
                for interface in interfaces
                if interface.fqcn in behavior_owners
            ),
            None,
        )
        if interface_owner is None:
            raise BlockRegistryCatalogExtractionError(
                f"{matches[0].source_path}: {factory_class} implements "
                f"{interface_fqcn} without resolving {method_name}"
            )
        owners[factory_class] = interface_owner.simple_name
    return owners


def extract_block_use_item_on_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "useItemOn",
        "InteractionResult",
        (
            "ItemStack",
            "BlockState",
            "Level",
            "BlockPos",
            "Player",
            "InteractionHand",
            "BlockHitResult",
        ),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_use_without_item_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "useWithoutItem",
        "InteractionResult",
        ("BlockState", "Level", "BlockPos", "Player", "BlockHitResult"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_menu_provider_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve each block to the class owning its `getMenuProvider` override."""

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "getMenuProvider",
        "MenuProvider",
        ("BlockState", "Level", "BlockPos"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_analog_output_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve blocks with analog output to their nearest signal implementation."""

    has_analog_output_owner = _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "hasAnalogOutputSignal",
        "boolean",
        ("BlockState",),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )
    for owner in sorted(
        {owner for owner in has_analog_output_owner.values() if owner is not None}
    ):
        matches = [
            (source, method)
            for source in block_class_sources
            for method in source.methods
            if method.owner.rsplit(".", 1)[-1] == owner
            and _is_block_method(
                source,
                method,
                "hasAnalogOutputSignal",
                "boolean",
                ("BlockState",),
            )
        ]
        if len(matches) != 1:
            raise BlockRegistryCatalogExtractionError(
                f"expected exactly one {owner}.hasAnalogOutputSignal, got "
                f"{len(matches)}"
            )
        source, method = matches[0]
        body = method.node.child_by_field_name("body")
        statements = [] if body is None else body.named_children
        expression = (
            statements[0].named_children[0]
            if len(statements) == 1
            and statements[0].type == "return_statement"
            and len(statements[0].named_children) == 1
            else None
        )
        if expression is None or source.expression(expression).canonical != "true":
            raise BlockRegistryCatalogExtractionError(
                f"{source.relative_path}: {owner}.hasAnalogOutputSignal must "
                "remain the literal true override"
            )
    get_analog_output_owner = _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "getAnalogOutputSignal",
        "int",
        ("BlockState", "Level", "BlockPos", "Direction"),
        None,
    )
    return {
        factory_class: (
            get_analog_output_owner[factory_class]
            if has_analog_output_owner[factory_class] is not None
            else None
        )
        for factory_class in has_analog_output_owner
    }


def extract_block_redstone_signal_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> tuple[
    dict[str, str | None],
    dict[str, str | None],
    dict[str, str | None],
]:
    """Resolve the nearest vanilla redstone signal implementation owners."""

    default_owner = "net.minecraft.world.level.block.state.BlockBehaviour"
    is_signal_source_owner = _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "isSignalSource",
        "boolean",
        ("BlockState",),
        default_owner,
    )
    signal_owner = _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "getSignal",
        "int",
        ("BlockState", "BlockGetter", "BlockPos", "Direction"),
        default_owner,
    )
    direct_signal_owner = _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "getDirectSignal",
        "int",
        ("BlockState", "BlockGetter", "BlockPos", "Direction"),
        default_owner,
    )
    return is_signal_source_owner, signal_owner, direct_signal_owner


def extract_block_can_survive_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.canSurvive`` owners."""

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "canSurvive",
        "boolean",
        ("BlockState", "LevelReader", "BlockPos"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_random_tick_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.randomTick`` owners.

    ``BlockBehaviour.randomTick`` is an empty body, so ``None`` is "this block
    does nothing on a random tick" and a name is the class whose body the
    block reaches. This is a strictly finer statement than the block's
    ``randomTicks()`` registration flag, which only says whether the scheduler
    ever visits it.
    """

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "randomTick",
        "void",
        ("BlockState", "ServerLevel", "BlockPos", "RandomSource"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_tick_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.tick`` owners.

    ``tick`` is the *scheduled* tick -- the body ``ServerLevel`` runs when a
    block ticket comes due -- and is a different member from ``randomTick``
    even though the two share a signature. Its base declaration is empty
    (``BlockBehaviour.java:352-353``), so ``None`` is "a scheduled tick does
    nothing to this block".

    The match is on the signature alone, never on the modifier: 26.1 widens
    two of these overrides to ``public`` (``BrushableBlock.java:83`` and
    ``SnifferEggBlock.java:65``), and a ``protected``-only search would drop
    both.
    """

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "tick",
        "void",
        ("BlockState", "ServerLevel", "BlockPos", "RandomSource"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_neighbor_changed_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.neighborChanged`` owners.

    This is the block-update entry point: the body a block reaches when a
    neighbouring block changes. The base declaration is empty
    (``BlockBehaviour.java:166-169``), so ``None`` is "this block ignores its
    neighbours".
    """

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "neighborChanged",
        "void",
        ("BlockState", "Level", "BlockPos", "Block", "Orientation", "boolean"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_on_place_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.onPlace`` owners.

    ``onPlace`` runs once the new state is already in the level, which is
    where a redstone component seeds its first scheduled tick. The base
    declaration is empty (``BlockBehaviour.java:171-172``), so ``None`` is
    "placing this block starts nothing".

    Three 26.1 overrides are ``public`` rather than ``protected``
    (``BrushableBlock.java:63``, ``FrostedIceBlock.java:40`` and
    ``SnifferEggBlock.java:83``); matching the signature keeps them.
    """

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "onPlace",
        "void",
        ("BlockState", "Level", "BlockPos", "BlockState", "boolean"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_affect_neighbors_after_removal_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.affectNeighborsAfterRemoval`` owners.

    This is the removal half of the update pair: the body that runs once the
    block is gone, where a power source drops the signal it was feeding and a
    container spills its comparator reading. The base declaration is empty
    (``BlockBehaviour.java:174-175``), so ``None`` is "removing this block
    disturbs nothing".
    """

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "affectNeighborsAfterRemoval",
        "void",
        ("BlockState", "ServerLevel", "BlockPos", "boolean"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_may_place_on_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve ``VegetationBlock.mayPlaceOn`` owners.

    ``mayPlaceOn`` is declared by ``VegetationBlock`` itself rather than by
    ``BlockBehaviour``, so there is no default owner to fold away: a block
    outside that hierarchy resolves to ``None``, and a block inside it names
    the class whose body ``VegetationBlock.canSurvive`` reaches.
    """

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "mayPlaceOn",
        "boolean",
        ("BlockState", "BlockGetter", "BlockPos"),
        None,
    )


def extract_block_tag_resource_ids(source: JavaSource) -> dict[str, str]:
    """Map every ``BlockTags`` constant onto the tag resource id it creates.

    ``BlockTags.create`` is ``TagKey.create(Registries.BLOCK,
    Identifier.withDefaultNamespace(name))``, so the constant's resource id is
    its one string argument in the ``minecraft`` namespace.
    """

    tags: dict[str, str] = {}
    for field in source.fields_for(BLOCK_TAGS_OWNER):
        if field.type_spelling.replace(" ", "") != BLOCK_TAG_TYPE_SPELLING:
            continue
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or source.expression(field.node)
            _fail(source, expression, f"BlockTags.{field.name} lacks a create call")
        name = initializer.node.child_by_field_name("name")
        arguments = _arguments(source, initializer)
        if name is None or source.text(name) != "create" or len(arguments) != 1:
            _fail(source, initializer, f"BlockTags.{field.name} is not create(name)")
        tags[field.name] = f"minecraft:{_string(source, arguments[0])}"
    if not tags:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: no block tag constants found"
        )
    return tags


def _parameter_parts(source: JavaSource, parameter: JavaExpression) -> tuple[str, str]:
    """One formal parameter's declared type spelling and name."""

    type_node = parameter.node.child_by_field_name("type")
    name_node = parameter.node.child_by_field_name("name")
    if type_node is None or name_node is None:
        _fail(source, parameter, "formal parameter is incomplete")
    return source.text(type_node).replace(" ", ""), source.text(name_node)


def _may_place_on_support_field(source: JavaSource, block_type) -> str | None:
    """The instance field a class's ``mayPlaceOn`` tests, when that is its whole body.

    Recognizes exactly ``return <state>.is(this.<field>);``, the shape
    ``AttachedStemBlock``, ``StemBlock``, ``NetherRootsBlock`` and
    ``NetherFungusBlock`` share; every other body — including
    ``VegetationBlock``'s own ``state.is(BlockTags.SUPPORTS_VEGETATION)`` —
    answers ``None`` because it names no constructor-supplied tag.
    """

    for method in source.methods_for(block_type):
        if not _is_block_method(
            source, method, "mayPlaceOn", "boolean", MAY_PLACE_ON_PARAMETER_TYPES
        ):
            continue
        body = method.node.child_by_field_name("body")
        if body is None:
            return None
        statements = [
            child
            for child in body.named_children
            if child.type not in COMMENT_NODE_KINDS
        ]
        if len(statements) != 1 or statements[0].type != "return_statement":
            return None
        values = [
            child
            for child in statements[0].named_children
            if child.type not in COMMENT_NODE_KINDS
        ]
        if len(values) != 1:
            return None
        state_parameter = _parameter_parts(source, method.parameters[0])[1]
        match = re.fullmatch(
            re.escape(state_parameter) + r"\.is\(this\.(\w+)\)",
            source.expression(values[0]).canonical.replace(" ", ""),
        )
        return match.group(1) if match else None
    return None


def _support_tag_constructor_index(
    source: JavaSource, block_type, field_name: str
) -> int:
    """The constructor parameter position that stores ``this.<field_name>``.

    This is what distinguishes ``StemBlock``'s two ``TagKey<Block>``
    parameters: ``mayPlaceOn`` names ``stemSupportBlocks``, and only the
    parameter assigned to that field counts.
    """

    constructors = [method for method in source.methods_for(block_type) if method.is_constructor]
    if len(constructors) != 1:
        raise BlockRegistryCatalogExtractionError(
            f"{block_type.simple_name} has {len(constructors)} constructors, "
            "so its support tag argument is ambiguous"
        )
    constructor = constructors[0]
    assigned: str | None = None
    for node in descendants(constructor.node):
        if node.type != "assignment_expression":
            continue
        left = node.child_by_field_name("left")
        right = node.child_by_field_name("right")
        if left is None or right is None:
            continue
        if source.expression(left).canonical.replace(" ", "") != f"this.{field_name}":
            continue
        assigned = source.expression(right).canonical.replace(" ", "")
    if assigned is None:
        raise BlockRegistryCatalogExtractionError(
            f"{block_type.simple_name}.{field_name} is never assigned from a constructor parameter"
        )
    for index, parameter in enumerate(constructor.parameters):
        type_spelling, name = _parameter_parts(source, parameter)
        if name != assigned:
            continue
        if type_spelling != BLOCK_TAG_TYPE_SPELLING:
            raise BlockRegistryCatalogExtractionError(
                f"{block_type.simple_name}.{field_name} is a "
                f"{type_spelling}, not a {BLOCK_TAG_TYPE_SPELLING}"
            )
        return index
    raise BlockRegistryCatalogExtractionError(
        f"{block_type.simple_name}.{field_name} is not a constructor parameter"
    )


def extract_block_may_place_on_support_tags(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    block_tags_source: JavaSource,
) -> dict[str, str | None]:
    """Resolve the ``BlockTags`` constant each registration hands ``mayPlaceOn``.

    ``mayPlaceOn`` owners such as ``NetherRootsBlock`` test a
    ``TagKey<Block>`` the *registration* supplies rather than one the class
    names, so the class tag alone cannot answer which blocks support them.
    This walks the three mechanical steps: the field ``mayPlaceOn`` tests, the
    constructor parameter that field is assigned from, and the argument at
    that position in ``Blocks.java``.
    """

    tag_resource_ids = extract_block_tag_resource_ids(block_tags_source)
    types = [item for source in block_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_simple_name: dict[str, list] = {}
    source_by_fqcn: dict[str, JavaSource] = {}
    for source in block_class_sources:
        for item in source.types:
            by_simple_name.setdefault(item.simple_name, []).append(item)
            source_by_fqcn[item.fqcn] = source

    argument_index_by_factory: dict[str, int] = {}
    for factory_class in sorted(
        {block_factory_class(declaration) for declaration in declarations}
    ):
        matches = by_simple_name.get(factory_class, [])
        if len(matches) != 1:
            raise BlockRegistryCatalogExtractionError(
                f"expected exactly one block class {factory_class}, got {len(matches)}"
            )
        block_type = matches[0]
        source = source_by_fqcn[block_type.fqcn]
        field_name = _may_place_on_support_field(source, block_type)
        if field_name is None:
            continue
        argument_index_by_factory[factory_class] = _support_tag_constructor_index(
            source, block_type, field_name
        )

    support_tags: dict[str, str | None] = {}
    for declaration in declarations:
        factory_class = block_factory_class(declaration)
        index = argument_index_by_factory.get(factory_class)
        if index is None:
            support_tags[declaration["field"]] = None
            continue
        arguments = _factory_creation_arguments(declaration, factory_class)
        if index >= len(arguments):
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: {factory_class} constructor has "
                f"{len(arguments)} arguments, no support tag at {index}"
            )
        canonical = arguments[index].get("canonical", "").replace(" ", "")
        owner, separator, constant = canonical.partition(".")
        if owner != "BlockTags" or not separator or constant not in tag_resource_ids:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: support tag argument "
                f"{canonical!r} is not a BlockTags constant"
            )
        support_tags[declaration["field"]] = tag_resource_ids[constant]
    if not any(tag is not None for tag in support_tags.values()):
        raise BlockRegistryCatalogExtractionError(
            "no registration supplies a mayPlaceOn support tag"
        )
    return support_tags


def _constructor_field_parameter_indices(
    source: JavaSource, block_type, type_spelling: str
) -> dict[str, int]:
    """Every ``this.<field> = <parameter>`` a class's constructor performs for a
    parameter of ``type_spelling``, as field name onto parameter position.

    This is :func:`_support_tag_constructor_index` turned around: instead of
    asking where one already-known field comes from, it enumerates every field
    of one type the constructor stores, so a registration argument can be read
    without the class naming which method consumes it.

    A class with no parameter of that type answers an empty map. A class that
    has one and more than one constructor is an error rather than a guess,
    because the registration argument list would no longer name a single
    position.
    """

    constructors = [
        method for method in source.methods_for(block_type) if method.is_constructor
    ]
    parameters_of_type = [
        name
        for constructor in constructors
        for parameter in constructor.parameters
        for spelling, name in [_parameter_parts(source, parameter)]
        if spelling == type_spelling
    ]
    if not parameters_of_type:
        return {}
    if len(constructors) != 1:
        raise BlockRegistryCatalogExtractionError(
            f"{block_type.simple_name} has {len(constructors)} constructors, so its "
            f"{type_spelling} arguments are ambiguous"
        )
    constructor = constructors[0]
    index_by_parameter: dict[str, int] = {}
    for index, parameter in enumerate(constructor.parameters):
        spelling, name = _parameter_parts(source, parameter)
        if spelling == type_spelling:
            index_by_parameter[name] = index

    indices: dict[str, int] = {}
    for node in descendants(constructor.node):
        if node.type != "assignment_expression":
            continue
        left = node.child_by_field_name("left")
        right = node.child_by_field_name("right")
        if left is None or right is None:
            continue
        target = source.expression(left).canonical.replace(" ", "")
        value = source.expression(right).canonical.replace(" ", "")
        if not target.startswith("this."):
            continue
        index = index_by_parameter.get(value)
        if index is not None:
            indices[target[len("this.") :]] = index
    return indices


def _factory_creation_arguments(declaration: dict, factory_class: str) -> list[dict]:
    """The argument facts of a registration's one ``new <factory_class>(...)``."""

    creations = [
        fact
        for fact in _walk_expression_facts(declaration["initializer"])
        if fact.get("kind") == "object_creation_expression"
        and fact.get("name") == factory_class
    ]
    if len(creations) != 1:
        raise BlockRegistryCatalogExtractionError(
            f"Blocks.{declaration['field']}: expected exactly one "
            f"{factory_class} constructor, got {len(creations)}"
        )
    return creations[0].get("arguments", [])


def _constructor_registry_key_arguments(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    type_spelling: str,
    constant_owner: str,
    resource_ids_by_constant: dict[str, str],
) -> dict[str, tuple[tuple[str, str], ...]]:
    """The registry ids a registration hands its block constructor, by field name.

    ``StemBlock``'s ``fruit``/``attachedStem``/``fruitSupportBlocks`` and
    ``AttachedStemBlock``'s ``stem``/``fruit`` are read by bodies that are far
    too big for the single-statement recognizer
    :func:`_may_place_on_support_field` uses, so nothing about *which* method
    consumes them is asserted here. What is mechanical is the pair of steps
    either side of the field: the constructor parameter it is assigned from,
    and the argument at that position in ``Blocks.java``.

    A class whose own constructor does not take the key answers nothing --
    ``GrassBlock`` passes ``BlockIds.DIRT`` to ``super`` rather than accepting
    it (``GrassBlock.java:29-31``) -- because the registration argument list
    holds no position for it.
    """

    types = [item for source in block_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_simple_name: dict[str, list] = {}
    source_by_fqcn: dict[str, JavaSource] = {}
    for source in block_class_sources:
        for item in source.types:
            by_simple_name.setdefault(item.simple_name, []).append(item)
            source_by_fqcn[item.fqcn] = source

    indices_by_factory: dict[str, dict[str, int]] = {}
    for factory_class in sorted(
        {block_factory_class(declaration) for declaration in declarations}
    ):
        matches = by_simple_name.get(factory_class, [])
        if len(matches) != 1:
            raise BlockRegistryCatalogExtractionError(
                f"expected exactly one block class {factory_class}, got {len(matches)}"
            )
        block_type = matches[0]
        indices = _constructor_field_parameter_indices(
            source_by_fqcn[block_type.fqcn], block_type, type_spelling
        )
        if indices:
            indices_by_factory[factory_class] = indices

    arguments_by_field: dict[str, tuple[tuple[str, str], ...]] = {}
    for declaration in declarations:
        factory_class = block_factory_class(declaration)
        indices = indices_by_factory.get(factory_class)
        if indices is None:
            arguments_by_field[declaration["field"]] = ()
            continue
        arguments = _factory_creation_arguments(declaration, factory_class)
        resolved: list[tuple[str, str]] = []
        for field_name, index in sorted(indices.items()):
            if index >= len(arguments):
                raise BlockRegistryCatalogExtractionError(
                    f"Blocks.{declaration['field']}: {factory_class} constructor has "
                    f"{len(arguments)} arguments, no {type_spelling} at {index}"
                )
            canonical = arguments[index].get("canonical", "").replace(" ", "")
            owner, separator, constant = canonical.partition(".")
            if (
                owner != constant_owner
                or not separator
                or constant not in resource_ids_by_constant
            ):
                raise BlockRegistryCatalogExtractionError(
                    f"Blocks.{declaration['field']}: {field_name} argument "
                    f"{canonical!r} is not a {constant_owner} constant"
                )
            resolved.append((field_name, resource_ids_by_constant[constant]))
        arguments_by_field[declaration["field"]] = tuple(resolved)
    if not any(arguments_by_field.values()):
        raise BlockRegistryCatalogExtractionError(
            f"no registration supplies a {type_spelling} constructor argument"
        )
    return arguments_by_field


def extract_block_constructor_block_ids(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    block_ids_source: JavaSource,
) -> dict[str, tuple[tuple[str, str], ...]]:
    """Every ``ResourceKey<Block>`` a registration hands its block constructor."""

    return _constructor_registry_key_arguments(
        declarations,
        block_class_sources,
        "ResourceKey<Block>",
        "BlockIds",
        {
            constant: f"minecraft:{path}"
            for constant, path in extract_block_id_constants(block_ids_source).items()
        },
    )


def extract_block_constructor_block_tags(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    block_tags_source: JavaSource,
) -> dict[str, tuple[tuple[str, str], ...]]:
    """Every ``TagKey<Block>`` a registration hands its block constructor."""

    return _constructor_registry_key_arguments(
        declarations,
        block_class_sources,
        BLOCK_TAG_TYPE_SPELLING,
        "BlockTags",
        extract_block_tag_resource_ids(block_tags_source),
    )


def extract_block_clone_item_stack_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.getCloneItemStack`` owners."""

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "getCloneItemStack",
        "ItemStack",
        ("LevelReader", "BlockPos", "BlockState", "boolean"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_get_state_for_placement_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``Block.getStateForPlacement`` owners."""

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "getStateForPlacement",
        "BlockState",
        ("BlockPlaceContext",),
        "net.minecraft.world.level.block.Block",
    )


def extract_block_bonemeal_target_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve the nearest client-visible ``BonemealableBlock`` target check."""

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "isValidBonemealTarget",
        "boolean",
        ("LevelReader", "BlockPos", "BlockState"),
        None,
    )


def extract_block_propagates_skylight_down_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.propagatesSkylightDown`` owners."""

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "propagatesSkylightDown",
        "boolean",
        ("BlockState",),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_visual_shape_override_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve non-default ``BlockBehaviour.getVisualShape`` owners."""

    return _extract_block_method_override_owners(
        declarations,
        block_class_sources,
        "getVisualShape",
        "VoxelShape",
        ("BlockState", "BlockGetter", "BlockPos", "CollisionContext"),
        "net.minecraft.world.level.block.state.BlockBehaviour",
    )


def extract_block_bucket_pickup_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve the nearest ``BucketPickup.pickupBlock`` implementation."""

    return _extract_block_interface_method_owners(
        declarations,
        block_class_sources,
        "net.minecraft.world.level.block.BucketPickup",
        "pickupBlock",
        "ItemStack",
        ("LivingEntity", "LevelAccessor", "BlockPos", "BlockState"),
    )


def extract_block_liquid_container_owners(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    """Resolve the nearest ``LiquidBlockContainer.canPlaceLiquid`` implementation."""

    return _extract_block_interface_method_owners(
        declarations,
        block_class_sources,
        "net.minecraft.world.level.block.LiquidBlockContainer",
        "canPlaceLiquid",
        "boolean",
        ("LivingEntity", "BlockGetter", "BlockPos", "BlockState", "Fluid"),
    )


def extract_block_replaceable_membership(
    declarations: Sequence[dict],
) -> dict[str, bool]:
    """Extract the cached ``BlockBehaviour.Properties.replaceable`` flag.

    Vanilla registrations opt into this flag through the fluent
    ``Properties.replaceable()`` call. Registrations rooted in copy/helper
    factories in 26.1 do not inherit a replaceable source, so the explicit
    call is the complete positive membership.
    """

    return {
        declaration["field"]: any(
            fact.get("kind") == "method_invocation"
            and fact.get("name") == "replaceable"
            for fact in _walk_expression_facts(declaration["initializer"])
        )
        for declaration in declarations
    }


def extract_block_loot_table_membership(
    declarations: Sequence[dict], blocks: JavaSource
) -> dict[str, bool]:
    """Resolve the optional loot table carried by block registration properties."""
    values: dict[str, bool] = {}
    helpers = {method.name: method for method in blocks.methods_for(BLOCKS_OWNER)}

    def block_value(fact: dict, env: dict[str, dict]) -> bool:
        name = fact["canonical"].removeprefix("Blocks.")
        if name in env:
            return block_value(env[name], {})
        if name not in values:
            raise BlockRegistryCatalogExtractionError(f"unresolved loot-table block {name}")
        return values[name]

    def table_value(fact: dict, env: dict[str, dict]) -> bool:
        if fact.get("name") != "getLootTable" or fact.get("object") is None:
            raise BlockRegistryCatalogExtractionError("unsupported loot-table override")
        return block_value(fact["object"], env)

    def evaluate(fact: dict, env: dict[str, dict]) -> bool:
        name = fact.get("name")
        args = fact.get("arguments", [])
        parent = fact.get("object")
        if fact["kind"] != "method_invocation":
            raise BlockRegistryCatalogExtractionError(
                f"unsupported loot properties {fact['canonical']}"
            )
        if name == "register":
            return evaluate(args[-1], env)
        if parent and parent["canonical"] == "BlockBehaviour.Properties":
            if name in {"of", "ofLegacyCopy"}:
                return True
            if name == "ofFullCopy":
                return block_value(args[0], env)
        if parent is None and name in helpers:
            method = helpers[name]
            local = {
                parameter.canonical.split()[-1]: env.get(argument["canonical"], argument)
                for parameter, argument in zip(method.parameters, args)
            }
            if name == "wallVariant":
                overrides = [
                    call
                    for call in blocks.calls
                    if method.span.start_byte <= call.span.start_byte
                    and call.span.end_byte <= method.span.end_byte
                    and call.name == "overrideLootTable"
                ]
                if len(overrides) != 1:
                    raise BlockRegistryCatalogExtractionError("unsupported wall loot-table helper")
                return table_value(expression_fact(blocks, overrides[0].arguments[0]), local)
            returns = [node for node in descendants(method.node) if node.type == "return_statement"]
            if len(returns) != 1 or len(returns[0].named_children) != 1:
                raise BlockRegistryCatalogExtractionError(
                    f"unsupported loot properties helper {name}"
                )
            returned = blocks.expression(returns[0].named_children[0])
            return evaluate(expression_fact(blocks, returned), local)
        if parent is not None:
            if name == "noLootTable":
                return False
            if name == "overrideLootTable":
                return table_value(args[0], env)
            return evaluate(parent, env)
        raise BlockRegistryCatalogExtractionError(f"unsupported loot properties call {name}")

    for declaration in declarations:
        initializer = declaration["initializer"]
        if declaration.get("group_variant"):
            initializer = initializer["arguments"][-1]["children"][-1]
        values[declaration["field"]] = evaluate(initializer, {})
    return values


def extract_concrete_powder_targets(
    declarations: Sequence[dict],
) -> dict[str, str]:
    """Extract each ``ConcretePowderBlock`` registration's concrete block."""

    fields = {declaration["field"] for declaration in declarations}
    targets: dict[str, str] = {}
    for declaration in declarations:
        if block_factory_class(declaration) != "ConcretePowderBlock":
            continue
        initializer = declaration["initializer"]
        arguments = initializer.get("arguments", [])
        if len(arguments) != 3:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: ConcretePowderBlock registration "
                f"has {len(arguments)} arguments"
            )
        creations = [
            fact
            for fact in _walk_expression_facts(arguments[1])
            if fact.get("kind") == "object_creation_expression"
            and fact.get("name") == "ConcretePowderBlock"
        ]
        if len(creations) != 1:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: expected exactly one "
                f"ConcretePowderBlock constructor, got {len(creations)}"
            )
        constructor_arguments = creations[0].get("arguments", [])
        if len(constructor_arguments) != 2:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: ConcretePowderBlock constructor "
                f"has {len(constructor_arguments)} arguments"
            )
        concrete = constructor_arguments[0]
        if concrete.get("kind") != "identifier":
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: concrete target is not a Blocks field"
            )
        target = concrete.get("canonical")
        if target not in fields:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: unresolved concrete target {target!r}"
            )
        targets[declaration["field"]] = target
    if len(set(targets.values())) != len(targets):
        raise BlockRegistryCatalogExtractionError(
            "ConcretePowderBlock registrations do not have unique concrete targets"
        )
    return targets


DYE_COLOR_OWNER = "net.minecraft.world.item.DyeColor"
BEACON_BEAM_BLOCK_FQCN = "net.minecraft.world.level.block.BeaconBeamBlock"


def _dye_color_constant_from_facts(expression: dict) -> str | None:
    """The single ``DyeColor.<CONSTANT>`` argument of a block registration."""
    constants = {
        fact["canonical"].split(".", 1)[1]
        for fact in _walk_expression_facts(expression)
        if fact.get("kind") in {"field_access", "scoped_identifier"}
        and fact.get("canonical", "").startswith("DyeColor.")
    }
    if len(constants) > 1:
        raise BlockRegistryCatalogExtractionError(
            f"block registration mentions {len(constants)} dye colors"
        )
    return next(iter(constants), None)


def _returned_dye_color_constant(
    source: JavaSource, block_type, method_name: str
) -> str | None:
    """The constant returned by a no-argument ``DyeColor`` accessor."""
    for method in source.methods_for(block_type):
        if method.name != method_name or method.parameters:
            continue
        returns = [
            node
            for node in descendants(method.node)
            if node.type == "return_statement"
        ]
        if len(returns) != 1:
            return None
        value = returns[0].named_children[0] if returns[0].named_children else None
        if value is None:
            return None
        canonical = source.expression(value).canonical.replace(" ", "")
        if not canonical.startswith("DyeColor."):
            return None
        return canonical.split(".", 1)[1]
    return None


def extract_beacon_beam_colors(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str]:
    """Map every registered ``BeaconBeamBlock`` field to its ``DyeColor``.

    `BeaconBlockEntity.tick` reads `BeaconBeamBlock.getColor()` for each block
    above a beacon, so the beam colour is exactly this interface's value: an
    explicit registration argument where the block stores one, otherwise the
    constant its `getColor()` returns.
    """
    membership = extract_block_interface_membership(
        declarations, block_class_sources, BEACON_BEAM_BLOCK_FQCN
    )
    types_by_simple_name: dict[str, tuple[JavaSource, Any]] = {}
    for source in block_class_sources:
        for block_type in source.types:
            types_by_simple_name.setdefault(block_type.simple_name, (source, block_type))

    colors: dict[str, str] = {}
    for declaration in declarations:
        factory_class = block_factory_class(declaration)
        if not membership.get(factory_class):
            continue
        color = _dye_color_constant_from_facts(declaration["initializer"])
        if color is None:
            entry = types_by_simple_name.get(factory_class)
            if entry is not None:
                color = _returned_dye_color_constant(entry[0], entry[1], "getColor")
        if color is None:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: unresolved beacon beam dye color"
            )
        colors[declaration["field"]] = color
    if not colors:
        raise BlockRegistryCatalogExtractionError("no beacon beam blocks resolved")
    return colors


def extract_block_subclass_membership(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    target_fqcn: str,
) -> dict[str, bool]:
    """Resolve each registered factory class against one authoritative base class."""
    types = [item for source in block_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_fqcn = {item.fqcn: item for item in types}
    by_simple_name: dict[str, list] = {}
    for item in types:
        by_simple_name.setdefault(item.simple_name, []).append(item)
    if target_fqcn not in by_fqcn:
        raise BlockRegistryCatalogExtractionError(
            f"unresolved block membership target {target_fqcn}"
        )

    memberships: dict[str, bool] = {}
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
            if block_type.fqcn == target_fqcn:
                memberships[factory_class] = True
                break
            if (
                block_type.superclass is None
                or block_type.superclass not in by_fqcn
            ):
                memberships[factory_class] = False
                break
            block_type = by_fqcn[block_type.superclass]
        else:
            raise BlockRegistryCatalogExtractionError(
                f"superclass cycle while resolving {factory_class} membership"
            )
    return memberships


def extract_block_interface_membership(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
    target_fqcn: str,
) -> dict[str, bool]:
    """Resolve each registered factory class against one authoritative interface."""
    types = [item for source in block_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_fqcn = {item.fqcn: item for item in types}
    by_simple_name: dict[str, list] = {}
    for item in types:
        by_simple_name.setdefault(item.simple_name, []).append(item)
    if target_fqcn not in by_fqcn:
        raise BlockRegistryCatalogExtractionError(
            f"unresolved block interface membership target {target_fqcn}"
        )

    memberships: dict[str, bool] = {}
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
        class_chain = []
        seen_classes = set()
        while block_type.fqcn not in seen_classes:
            seen_classes.add(block_type.fqcn)
            class_chain.append(block_type)
            if (
                block_type.superclass is None
                or block_type.superclass not in by_fqcn
            ):
                break
            block_type = by_fqcn[block_type.superclass]
        else:
            raise BlockRegistryCatalogExtractionError(
                f"superclass cycle while resolving {factory_class} interface membership"
            )

        pending = [
            interface
            for class_type in class_chain
            for interface in class_type.interfaces
        ]
        seen_interfaces = set()
        is_member = False
        while pending:
            interface = pending.pop(0)
            if interface in seen_interfaces:
                continue
            seen_interfaces.add(interface)
            if interface == target_fqcn:
                is_member = True
                break
            interface_type = by_fqcn.get(interface)
            if interface_type is not None:
                pending.extend(interface_type.interfaces)
        memberships[factory_class] = is_member
    return memberships


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


CHANGE_OVER_TIME_BLOCK_FQCN = "net.minecraft.world.level.block.ChangeOverTimeBlock"
WEATHERING_COPPER_FQCN = "net.minecraft.world.level.block.WeatheringCopper"
WEATHERING_COPPER_PATH = "net/minecraft/world/level/block/WeatheringCopper.java"
WEATHER_STATE_SIMPLE_NAME = "WeatherState"
WEATHER_STATE_PREFIX = "WeatheringCopper.WeatherState."
NEXT_BY_BLOCK_FIELD = "NEXT_BY_BLOCK"
WEATHERING_MAPPING_METHOD = "weatheringMapping"
# Every method the `NEXT_BY_BLOCK` initializer is allowed to invoke. An
# unlisted one means the map is being built some other way and the pairs read
# out of it would be a guess.
NEXT_BY_BLOCK_INVOCATIONS = {
    "memoize",
    "builder",
    "build",
    "put",
    "putAll",
    WEATHERING_MAPPING_METHOD,
}
# The three variants `WeatheringCopperBlocks.weatheringMapping` pairs up, in
# the order that record spells them.
WEATHERING_MAPPING_VARIANTS = ("unaffected", "exposed", "weathered", "oxidized")


def _block_class_source(
    block_class_sources: Sequence[JavaSource], relative_path: str
) -> JavaSource:
    matches = [
        source for source in block_class_sources if source.relative_path == relative_path
    ]
    if len(matches) != 1:
        raise BlockRegistryCatalogExtractionError(
            f"expected exactly one {relative_path}, got {len(matches)}"
        )
    return matches[0]


def _nested_type(source: JavaSource, simple_name: str) -> JavaType:
    matches = [item for item in source.types if item.simple_name == simple_name]
    if len(matches) != 1:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: expected exactly one {simple_name}, got "
            f"{len(matches)}"
        )
    return matches[0]


def _enum_ordinals(source: JavaSource, java_type: JavaType) -> dict[str, int]:
    """One enum's constants as name to ordinal, in declaration order."""
    names: list[str] = []
    for node in type_body_members(java_type.body_node):
        if node.type != "enum_constant":
            continue
        name_node = node.child_by_field_name("name")
        if name_node is None:
            _fail(source, source.expression(node), "enum constant lacks a name")
        names.append(source.text(name_node))
    if not names or len(names) != len(set(names)):
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: {java_type.simple_name} has no or duplicate "
            "constants"
        )
    return {name: ordinal for ordinal, name in enumerate(names)}


def _weather_state_constant(declaration: dict) -> str | None:
    """The single `WeatheringCopper.WeatherState` constant a registration names."""
    constants = {
        fact["canonical"].replace(" ", "")[len(WEATHER_STATE_PREFIX) :]
        for fact in _walk_expression_facts(declaration["initializer"])
        if fact.get("kind") in {"field_access", "scoped_identifier"}
        and fact.get("canonical", "").replace(" ", "").startswith(WEATHER_STATE_PREFIX)
    }
    if len(constants) > 1:
        raise BlockRegistryCatalogExtractionError(
            f"Blocks.{declaration['field']}: registration names "
            f"{len(constants)} weather states"
        )
    return next(iter(constants), None)


def extract_change_over_time_ages(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, int]:
    """`ChangeOverTimeBlock.getAge().ordinal()` for every block that is one.

    The age is a constructor argument rather than a block-state property, so it
    is read where the registration hands it over: either the explicit
    ``WeatheringCopper.WeatherState`` constant, or -- for the four weathering
    members of a ``WeatheringCopperBlocks`` group -- the positional variant
    ``WeatheringCopperBlocks.create`` supplies.

    The generator also asserts here that ``ChangeOverTimeBlock`` and
    ``WeatheringCopper`` name the same set of blocks, which is what makes
    ``ChangeOverTimeBlock.getNextState``'s ``getAge().getClass()`` comparison a
    comparison every member passes.
    """

    source = _block_class_source(block_class_sources, WEATHERING_COPPER_PATH)
    ordinals = _enum_ordinals(source, _nested_type(source, WEATHER_STATE_SIMPLE_NAME))
    change_over_time = extract_block_interface_membership(
        declarations, block_class_sources, CHANGE_OVER_TIME_BLOCK_FQCN
    )
    weathering_copper = extract_block_interface_membership(
        declarations, block_class_sources, WEATHERING_COPPER_FQCN
    )
    if change_over_time != weathering_copper:
        raise BlockRegistryCatalogExtractionError(
            "ChangeOverTimeBlock and WeatheringCopper no longer name the same blocks"
        )
    ages: dict[str, int] = {}
    for declaration in declarations:
        if not change_over_time[block_factory_class(declaration)]:
            continue
        group_variant = declaration.get("group_variant")
        if group_variant is not None:
            constant = group_variant.upper()
        else:
            constant = _weather_state_constant(declaration)
        if constant is None or constant not in ordinals:
            raise BlockRegistryCatalogExtractionError(
                f"Blocks.{declaration['field']}: unresolved weather state "
                f"{constant!r}"
            )
        ages[declaration["field"]] = ordinals[constant]
    if not ages:
        raise BlockRegistryCatalogExtractionError(
            "no ChangeOverTimeBlock registrations were resolved"
        )
    return ages


def _blocks_field(source: JavaSource, expression: JavaExpression, description: str) -> str:
    constant = field_constant(source, expression, "Blocks")
    if constant is None:
        _fail(source, expression, f"{description} is not a Blocks constant")
    return constant


def extract_weathering_next_blocks(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, str]:
    """`WeatheringCopper.NEXT_BY_BLOCK` as registration field to registration field.

    Both spellings the builder uses are read: the explicit
    ``put(Blocks.A, Blocks.B)`` pairs and the three pairs each
    ``putAll(Blocks.G.weatheringMapping())`` contributes
    (``WeatheringCopperBlocks.java:47-49``).
    """

    source = _block_class_source(block_class_sources, WEATHERING_COPPER_PATH)
    fields_by_name = {declaration["field"]: declaration for declaration in declarations}
    initializer = None
    for field in source.fields_for(WEATHERING_COPPER_FQCN):
        if field.name == NEXT_BY_BLOCK_FIELD:
            initializer = field.initializer
    if initializer is None:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: {NEXT_BY_BLOCK_FIELD} has no initializer"
        )
    pairs: list[tuple[str, str]] = []
    for node in descendants(initializer.node):
        if node.type != "method_invocation":
            continue
        name_node = node.child_by_field_name("name")
        arguments_node = node.child_by_field_name("arguments")
        if name_node is None or arguments_node is None:
            _fail(source, source.expression(node), "incomplete map builder call")
        name = source.text(name_node)
        if name not in NEXT_BY_BLOCK_INVOCATIONS:
            _fail(
                source,
                source.expression(node),
                f"unsupported {NEXT_BY_BLOCK_FIELD} builder call {name}",
            )
        arguments = [source.expression(child) for child in arguments_node.named_children]
        if name == "put":
            if len(arguments) != 2:
                _fail(source, source.expression(node), "put does not take two blocks")
            pairs.append(
                (
                    _blocks_field(source, arguments[0], "weathering source"),
                    _blocks_field(source, arguments[1], "weathering target"),
                )
            )
        elif name == "putAll":
            if len(arguments) != 1:
                _fail(source, source.expression(node), "putAll does not take one map")
            mapping = arguments[0].node
            mapping_name = mapping.child_by_field_name("name")
            group = mapping.child_by_field_name("object")
            if (
                mapping.type != "method_invocation"
                or mapping_name is None
                or source.text(mapping_name) != WEATHERING_MAPPING_METHOD
                or group is None
            ):
                _fail(
                    source,
                    arguments[0],
                    f"putAll argument is not a {WEATHERING_MAPPING_METHOD}() call",
                )
            base = _blocks_field(source, source.expression(group), "weathering group")
            members = []
            for variant in WEATHERING_MAPPING_VARIANTS:
                prefix = next(
                    field_prefix
                    for candidate, _, field_prefix in WEATHERING_COPPER_VARIANTS
                    if candidate == variant
                )
                member = f"{prefix}{base}"
                declaration = fields_by_name.get(member)
                if declaration is None or declaration.get("group_variant") != variant:
                    raise BlockRegistryCatalogExtractionError(
                        f"Blocks.{base}: {member} is not its {variant} member"
                    )
                members.append(member)
            pairs.extend(zip(members, members[1:], strict=False))
    unknown = [
        field for pair in pairs for field in pair if field not in fields_by_name
    ]
    if unknown:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: unresolved weathering blocks {sorted(unknown)}"
        )
    sources = [source_field for source_field, _ in pairs]
    targets = [target_field for _, target_field in pairs]
    if len(set(sources)) != len(sources) or len(set(targets)) != len(targets):
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: {NEXT_BY_BLOCK_FIELD} is not a bijection"
        )
    if not pairs:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: {NEXT_BY_BLOCK_FIELD} is empty"
        )
    return dict(pairs)


def extract_fire_block_odds(
    declarations: Sequence[dict],
    block_class_sources: Sequence[JavaSource],
) -> dict[str, tuple[int, int]]:
    """`FireBlock.bootStrap` ignite and burn odds by registered block field."""

    source = _block_class_source(block_class_sources, FIRE_BLOCK_PATH)
    fire_types = [item for item in source.types if item.fqcn == FIRE_BLOCK_FQCN]
    if len(fire_types) != 1:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: expected exactly one FireBlock, got {len(fire_types)}"
        )
    methods = [
        method
        for method in source.methods_for(fire_types[0])
        if method.name == "bootStrap" and not method.parameters
    ]
    if len(methods) != 1:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: expected exactly one FireBlock.bootStrap, got {len(methods)}"
        )
    known_fields = {declaration["field"] for declaration in declarations}
    odds: dict[str, tuple[int, int]] = {}
    for node in descendants(methods[0].node):
        if node.type != "method_invocation":
            continue
        name = node.child_by_field_name("name")
        if name is None or source.text(name) != "setFlammable":
            continue
        invocation = source.expression(node)
        arguments = _arguments(source, invocation)
        if len(arguments) != 3:
            _fail(source, invocation, "setFlammable does not take block and two odds")
        block = _blocks_field(source, arguments[0], "flammable block")
        if block not in known_fields:
            _fail(source, arguments[0], f"unresolved flammable block {block}")
        values: list[int] = []
        for argument, label in zip(arguments[1:], ("ignite", "burn"), strict=True):
            spelling = argument.canonical.replace("_", "")
            if not spelling.isdecimal():
                _fail(source, argument, f"{label} odds are not a decimal integer literal")
            value = int(spelling)
            if value <= 0:
                _fail(source, argument, f"{label} odds must be positive")
            values.append(value)
        if block in odds:
            _fail(source, arguments[0], f"duplicate flammable block {block}")
        odds[block] = (values[0], values[1])
    if not odds:
        raise BlockRegistryCatalogExtractionError(
            f"{source.relative_path}: FireBlock.bootStrap has no flammable blocks"
        )
    return odds
