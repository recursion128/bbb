"""Tree-sitter-backed extraction for pure vanilla item membership declarations."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import (
    JavaCall,
    JavaExpression,
    JavaMethod,
    JavaSource,
    SourceSpan,
    descendants,
)


class ItemMembershipCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class BrewingMembership:
    potion_slot_item_ids: tuple[str, ...]
    ingredient_item_ids: tuple[str, ...]


@dataclass(frozen=True)
class MenuMembership:
    enchantment_lapis_item_ids: tuple[str, ...]
    cartography_additional_item_ids: tuple[str, ...]


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise ItemMembershipCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected string literal resource path")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "expected non-empty string literal")
    return value


def _initializer_call(
    source: JavaSource, expression: JavaExpression
) -> tuple[str, tuple[JavaExpression, ...]]:
    if expression.node.type != "method_invocation":
        _fail(source, expression, "expected item field method invocation")
    name = expression.node.child_by_field_name("name")
    arguments = expression.node.child_by_field_name("arguments")
    if name is None or arguments is None:
        _fail(source, expression, "item field invocation lacks name or arguments")
    return source.text(name), tuple(
        source.expression(node) for node in arguments.named_children
    )


def _field_access(
    source: JavaSource, expression: JavaExpression, expected_owner: str
) -> str:
    if expression.node.type != "field_access":
        _fail(source, expression, f"expected {expected_owner} constant reference")
    owner = expression.node.child_by_field_name("object")
    field = expression.node.child_by_field_name("field")
    if owner is None or field is None or source.text(owner) != expected_owner:
        _fail(source, expression, f"expected {expected_owner} constant reference")
    return source.text(field)


def item_constant_resource_ids(
    items_source: JavaSource, blocks_source: JavaSource
) -> dict[str, str]:
    """Resolve only direct string/registerBlock item fields needed by consumers.

    This is deliberately not a second full item registry extractor. Unsupported
    unrelated fields stay outside the partial map; any referenced constant that
    is absent fails in `_resolve_item` with its source span.
    """
    block_resources: dict[str, str] = {}
    for field in blocks_source.fields_for("net.minecraft.world.level.block.Blocks"):
        if field.initializer is None or field.initializer.node.type != "method_invocation":
            continue
        name, arguments = _initializer_call(blocks_source, field.initializer)
        if name == "register" and arguments and arguments[0].node.type == "string_literal":
            block_resources[field.name] = "minecraft:" + _string(
                blocks_source, arguments[0]
            )

    resources: dict[str, str] = {}
    for field in items_source.fields_for("net.minecraft.world.item.Items"):
        if field.initializer is None or field.initializer.node.type != "method_invocation":
            continue
        name, arguments = _initializer_call(items_source, field.initializer)
        if (
            name == "registerItem"
            and arguments
            and arguments[0].node.type == "string_literal"
        ):
            resource_id = "minecraft:" + _string(items_source, arguments[0])
        elif name == "registerBlock" and arguments:
            block_name = _field_access(items_source, arguments[0], "Blocks")
            resource_id = block_resources.get(block_name)
            if resource_id is None:
                continue
        else:
            continue
        if field.name in resources:
            _fail(items_source, field.initializer, f"duplicate item constant {field.name}")
        resources[field.name] = resource_id
    return resources


def _method(source: JavaSource, owner: str, name: str) -> JavaMethod:
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == name and method.body is not None
    ]
    if len(methods) != 1:
        raise ItemMembershipCatalogExtractionError(
            f"{source.relative_path}: expected exactly one {owner}.{name} method with a body"
        )
    return methods[0]


def _parameter_name(source: JavaSource, method: JavaMethod, index: int) -> str:
    parameters = method.node.child_by_field_name("parameters")
    if parameters is None or len(parameters.named_children) <= index:
        raise ItemMembershipCatalogExtractionError(
            f"{source.relative_path}: {method.owner}.{method.name} lacks parameter {index}"
        )
    name = parameters.named_children[index].child_by_field_name("name")
    if name is None:
        raise ItemMembershipCatalogExtractionError(
            f"{source.relative_path}: {method.owner}.{method.name} parameter {index} lacks a name"
        )
    return source.text(name)


def _calls_in_body(
    source: JavaSource, body: SourceSpan, names: set[str]
) -> list[JavaCall]:
    calls = [
        call
        for call in source.calls
        if call.name in names
        and body.start_byte <= call.span.start_byte
        and call.span.end_byte <= body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    return calls


def _resolve_item(
    source: JavaSource, expression: JavaExpression, resources: dict[str, str]
) -> str:
    constant = _field_access(source, expression, "Items")
    resource_id = resources.get(constant)
    if resource_id is None:
        _fail(source, expression, f"Items.{constant} does not resolve to a registered item")
    return resource_id


def _unique(values: list[str]) -> tuple[str, ...]:
    return tuple(dict.fromkeys(values))


def brewing_membership(
    potion_brewing_source: JavaSource,
    brewing_menu_source: JavaSource,
    items_source: JavaSource,
    blocks_source: JavaSource,
    expected_potion_count: int = 4,
    expected_ingredient_count: int = 21,
) -> BrewingMembership:
    resources = item_constant_resource_ids(items_source, blocks_source)
    potion_method = _method(
        brewing_menu_source,
        "net.minecraft.world.inventory.BrewingStandMenu.PotionSlot",
        "mayPlaceItem",
    )
    potion_stack_name = _parameter_name(brewing_menu_source, potion_method, 0)
    potion_ids_list: list[str] = []
    assert potion_method.body is not None
    for call in _calls_in_body(brewing_menu_source, potion_method.body, {"is"}):
        if (
            call.qualifier is None
            or call.qualifier.canonical != potion_stack_name
            or len(call.arguments) != 1
        ):
            _fail(
                brewing_menu_source,
                brewing_menu_source.expression(call.node),
                "unsupported potion-slot item predicate",
            )
        potion_ids_list.append(
            _resolve_item(brewing_menu_source, call.arguments[0], resources)
        )
    potion_ids = _unique(potion_ids_list)

    mixes_method = _method(
        potion_brewing_source,
        "net.minecraft.world.item.alchemy.PotionBrewing",
        "addVanillaMixes",
    )
    builder_name = _parameter_name(potion_brewing_source, mixes_method, 0)
    ingredient_ids: list[str] = []
    assert mixes_method.body is not None
    for call in _calls_in_body(
        potion_brewing_source,
        mixes_method.body,
        {"addContainerRecipe", "addMix", "addStartMix"},
    ):
        expected_arity = 2 if call.name == "addStartMix" else 3
        argument_index = 0 if call.name == "addStartMix" else 1
        if (
            call.qualifier is None
            or call.qualifier.canonical != builder_name
            or len(call.arguments) != expected_arity
        ):
            _fail(
                potion_brewing_source,
                potion_brewing_source.expression(call.node),
                f"unsupported {call.name} ingredient declaration",
            )
        ingredient_ids.append(
            _resolve_item(
                potion_brewing_source, call.arguments[argument_index], resources
            )
        )
    ingredient_ids_tuple = _unique(ingredient_ids)

    if len(potion_ids) != expected_potion_count:
        raise ItemMembershipCatalogExtractionError(
            f"{brewing_menu_source.relative_path}: expected {expected_potion_count} potion-slot "
            f"items, extracted {len(potion_ids)}"
        )
    if len(ingredient_ids_tuple) != expected_ingredient_count:
        raise ItemMembershipCatalogExtractionError(
            f"{potion_brewing_source.relative_path}: expected {expected_ingredient_count} "
            f"brewing ingredients, extracted {len(ingredient_ids_tuple)}"
        )
    return BrewingMembership(potion_ids, ingredient_ids_tuple)


def _unique_item_predicate(
    source: JavaSource,
    resources: dict[str, str],
    expected_count: int,
) -> tuple[str, ...]:
    candidates: list[tuple[str, list[JavaCall]]] = []
    for node in descendants(source.root_node):
        if node.type != "method_declaration":
            continue
        name = node.child_by_field_name("name")
        body = node.child_by_field_name("body")
        parameters = node.child_by_field_name("parameters")
        if (
            name is None
            or source.text(name) != "mayPlace"
            or body is None
            or parameters is None
            or len(parameters.named_children) != 1
        ):
            continue
        parameter_name = parameters.named_children[0].child_by_field_name("name")
        if parameter_name is None:
            _fail(source, source.expression(node), "mayPlace parameter lacks a name")
        calls = _calls_in_body(source, source.span(body), {"is"})
        if calls:
            candidates.append((source.text(parameter_name), calls))
    if len(candidates) != 1:
        raise ItemMembershipCatalogExtractionError(
            f"{source.relative_path}: expected exactly one item-based mayPlace predicate, "
            f"found {len(candidates)}"
        )
    stack_name, calls = candidates[0]
    item_ids: list[str] = []
    for call in calls:
        if (
            call.qualifier is None
            or call.qualifier.canonical != stack_name
            or len(call.arguments) != 1
        ):
            _fail(source, source.expression(call.node), "unsupported menu item predicate")
        item_ids.append(_resolve_item(source, call.arguments[0], resources))
    unique_ids = _unique(item_ids)
    if len(unique_ids) != expected_count:
        raise ItemMembershipCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} menu predicate items, "
            f"extracted {len(unique_ids)}"
        )
    return unique_ids


def menu_membership(
    enchantment_menu_source: JavaSource,
    cartography_menu_source: JavaSource,
    items_source: JavaSource,
    blocks_source: JavaSource,
    expected_lapis_count: int = 1,
    expected_cartography_count: int = 3,
) -> MenuMembership:
    resources = item_constant_resource_ids(items_source, blocks_source)
    return MenuMembership(
        enchantment_lapis_item_ids=_unique_item_predicate(
            enchantment_menu_source,
            resources,
            expected_lapis_count,
        ),
        cartography_additional_item_ids=_unique_item_predicate(
            cartography_menu_source,
            resources,
            expected_cartography_count,
        ),
    )
