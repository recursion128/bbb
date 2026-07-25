"""Tree-sitter-backed vanilla item registry declaration extraction.

This module owns only the Minecraft item registration vocabulary. Java lexing,
declaration discovery, delimiter handling, and malformed-CST diagnostics remain
owned by :mod:`vanilla_java_source`.
"""

from __future__ import annotations

import json
import math
import struct
from dataclasses import dataclass
from typing import Any, Sequence

from vanilla_java_source import (
    JavaExpression,
    JavaSource,
    expression_fact,
    resolve_class_hierarchy,
)


class ItemRegistryCatalogExtractionError(ValueError):
    pass


ITEMS_OWNER = "net.minecraft.world.item.Items"
ITEM_OWNER = "net.minecraft.world.item.Item"
ITEM_IDS_OWNER = "net.minecraft.references.ItemIds"
FOODS_OWNER = "net.minecraft.world.food.Foods"
ITEM_FIELD_TYPES = {"Item", "WeatheringCopperItems"}


def _walk_expression_facts(expression: dict):
    yield expression
    object_fact = expression.get("object")
    if object_fact is not None:
        yield from _walk_expression_facts(object_fact)
    for key in ("arguments", "children"):
        for child in expression.get(key, []):
            yield from _walk_expression_facts(child)


def item_factory_class(declaration: dict) -> str:
    initializer = declaration["initializer"]
    name = initializer.get("name")
    arguments = initializer.get("arguments", [])
    if declaration["field_type"] == "WeatheringCopperItems":
        return "BlockItem"
    if name == "registerSpawnEgg":
        return "SpawnEggItem"
    if name not in {"registerBlock", "registerItem"}:
        raise ItemRegistryCatalogExtractionError(
            f"Items.{declaration['field']}: unsupported factory root {name!r}"
        )

    for argument in arguments[1:]:
        if (
            argument.get("kind") == "method_invocation"
            and argument.get("name") == "createBlockItemWithCustomItemName"
        ):
            return "BlockItem"
        for fact in _walk_expression_facts(argument):
            if fact.get("kind") == "method_reference":
                owner, separator, constructor = fact["canonical"].partition("::")
                if separator and constructor == "new":
                    return owner.rsplit(".", 1)[-1]
            if fact.get("kind") == "object_creation_expression":
                class_name = (
                    fact.get("name", "").rsplit(".", 1)[-1].split("<", 1)[0]
                )
                if class_name.endswith("Item"):
                    return class_name

    return "BlockItem" if name == "registerBlock" else "Item"


def _is_item_behavior_method(
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


def _extract_item_behavior_override_owners(
    declarations: Sequence[dict],
    item_class_sources: Sequence[JavaSource],
    method_name: str,
    parameter_types: tuple[str, ...],
) -> dict[str, str | None]:
    types = [item for source in item_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_fqcn = {item.fqcn: item for item in types}
    by_simple_name: dict[str, list] = {}
    for item in types:
        by_simple_name.setdefault(item.simple_name, []).append(item)
    behavior_owners = {
        method.owner
        for source in item_class_sources
        for method in source.methods
        if _is_item_behavior_method(
            source, method, method_name, parameter_types
        )
    }

    owners: dict[str, str | None] = {}
    for factory_class in sorted({item_factory_class(item) for item in declarations}):
        matches = by_simple_name.get(factory_class, [])
        if len(matches) != 1:
            raise ItemRegistryCatalogExtractionError(
                f"expected exactly one item class {factory_class}, got "
                f"{[item.fqcn for item in matches]}"
            )
        item_type = matches[0]
        seen = set()
        while item_type.fqcn not in seen:
            seen.add(item_type.fqcn)
            if item_type.fqcn in behavior_owners:
                owners[factory_class] = (
                    None if item_type.simple_name == "Item" else item_type.simple_name
                )
                break
            if item_type.superclass is None or item_type.superclass not in by_fqcn:
                raise ItemRegistryCatalogExtractionError(
                    f"{item_type.source_path}: {factory_class} does not resolve to "
                    f"Item.{method_name}"
                )
            item_type = by_fqcn[item_type.superclass]
        else:
            raise ItemRegistryCatalogExtractionError(
                f"superclass cycle while resolving {factory_class}.use"
            )
    return owners


def extract_item_use_override_owners(
    declarations: Sequence[dict],
    item_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    return _extract_item_behavior_override_owners(
        declarations,
        item_class_sources,
        "use",
        ("Level", "Player", "InteractionHand"),
    )


def extract_item_use_on_override_owners(
    declarations: Sequence[dict],
    item_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    return _extract_item_behavior_override_owners(
        declarations,
        item_class_sources,
        "useOn",
        ("UseOnContext",),
    )


def extract_item_interact_living_entity_override_owners(
    declarations: Sequence[dict],
    item_class_sources: Sequence[JavaSource],
) -> dict[str, str | None]:
    return _extract_item_behavior_override_owners(
        declarations,
        item_class_sources,
        "interactLivingEntity",
        ("ItemStack", "Player", "LivingEntity", "InteractionHand"),
    )


def _unique_field(source: JavaSource, owner: str, name: str):
    fields = [
        field
        for field in source.fields_for(owner)
        if field.name == name and field.initializer is not None
    ]
    if len(fields) != 1:
        raise ItemRegistryCatalogExtractionError(
            f"{source.relative_path}: expected exactly one {owner}.{name} field"
        )
    return fields[0]


def _calls_within(source: JavaSource, owner: str, span, name: str):
    return [
        call
        for call in source.calls_for(owner)
        if call.name == name
        and span.start_byte <= call.span.start_byte
        and call.span.end_byte <= span.end_byte
    ]


def _fact_field(fact: dict, owner: str) -> str | None:
    if fact.get("kind") != "field_access":
        return None
    object_fact = fact.get("object")
    if object_fact is None or object_fact.get("canonical") != owner:
        return None
    return fact.get("name")


def _block_constant(source: JavaSource, expression: JavaExpression) -> str | None:
    fact = expression_fact(source, expression)
    direct = _fact_field(fact, "Blocks")
    if direct is not None:
        return direct
    if fact.get("kind") == "method_invocation" and fact.get("name") == "defaultBlockState":
        object_fact = fact.get("object")
        if object_fact is not None:
            return _fact_field(object_fact, "Blocks")
    return None


def _map_put_pairs(source: JavaSource, owner: str, field_name: str):
    field = _unique_field(source, owner, field_name)
    pairs = []
    for call in _calls_within(source, owner, field.initializer.span, "put"):
        if len(call.arguments) != 2:
            _fail(source, field.initializer, "map put must have two arguments")
        source_block = _block_constant(source, call.arguments[0])
        target_block = _block_constant(source, call.arguments[1])
        if source_block is None or target_block is None:
            _fail(source, field.initializer, "map put must use Blocks constants")
        pairs.append((source_block, target_block))
    return field, pairs


def _camel_to_snake(value: str) -> str:
    result = []
    for character in value:
        if character.isupper() and result:
            result.append("_")
        result.append(character.lower())
    return "".join(result)


def _weathering_group_relations(
    source: JavaSource,
    method_name: str,
) -> list[tuple[str, str]]:
    owner = "net.minecraft.world.level.block.WeatheringCopperBlocks"
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == method_name and method.body is not None
    ]
    if len(methods) != 1:
        raise ItemRegistryCatalogExtractionError(
            f"{source.relative_path}: expected exactly one "
            f"WeatheringCopperBlocks.{method_name}"
        )
    method = methods[0]
    calls = _calls_within(source, owner, method.body, "of")
    calls = [call for call in calls if len(call.arguments) >= 2]
    if len(calls) != 1 or len(calls[0].arguments) % 2:
        raise ItemRegistryCatalogExtractionError(
            f"{source.relative_path}: {method_name} must return one even map"
        )
    components = []
    for argument in calls[0].arguments:
        fact = expression_fact(source, argument)
        component = _fact_field(fact, "this")
        if component is None:
            _fail(source, argument, f"{method_name} must use record components")
        components.append(_camel_to_snake(component))
    return list(zip(components[::2], components[1::2], strict=True))


def _group_mapping_calls(
    source: JavaSource,
    owner: str,
    field,
    method_name: str,
) -> list[str]:
    groups = []
    for call in _calls_within(source, owner, field.initializer.span, "putAll"):
        if len(call.arguments) != 1:
            _fail(source, field.initializer, "putAll must have one argument")
        fact = expression_fact(source, call.arguments[0])
        if fact.get("kind") != "method_invocation" or fact.get("name") != method_name:
            _fail(
                source,
                call.arguments[0],
                f"unsupported putAll in {field.name}; expected {method_name}",
            )
        object_fact = fact.get("object")
        group = _fact_field(object_fact or {}, "Blocks")
        if group is None:
            _fail(source, call.arguments[0], f"{method_name} must target Blocks group")
        groups.append(group)
    return groups


def _expanded_group_pairs(
    declarations: Sequence[dict],
    group: str,
    relations: Sequence[tuple[str, str]],
) -> list[tuple[str, str]]:
    base = next(
        (declaration for declaration in declarations if declaration["field"] == group),
        None,
    )
    if base is None or base.get("group_variant") is None:
        raise ItemRegistryCatalogExtractionError(
            f"unresolved WeatheringCopperBlocks group Blocks.{group}"
        )
    canonical = base["initializer"]["canonical"]
    variants = {
        declaration["group_variant"]: declaration["field"]
        for declaration in declarations
        if declaration.get("group_variant") is not None
        and declaration["initializer"]["canonical"] == canonical
    }
    pairs = []
    for source_variant, target_variant in relations:
        if source_variant not in variants or target_variant not in variants:
            raise ItemRegistryCatalogExtractionError(
                f"Blocks.{group} lacks {source_variant}->{target_variant}"
            )
        pairs.append((variants[source_variant], variants[target_variant]))
    return pairs


def _validate_transform_pairs(
    declarations: Sequence[dict],
    name: str,
    pairs: Sequence[tuple[str, str]],
) -> list[dict[str, str]]:
    block_fields = {declaration["field"] for declaration in declarations}
    seen = set()
    values = []
    for source, target in pairs:
        if source not in block_fields or target not in block_fields:
            raise ItemRegistryCatalogExtractionError(
                f"{name}: unresolved Blocks constant {source}->{target}"
            )
        if source in seen:
            raise ItemRegistryCatalogExtractionError(
                f"{name}: duplicate source Blocks.{source}"
            )
        seen.add(source)
        values.append({"source": source, "target": target})
    return sorted(values, key=lambda entry: entry["source"])


def extract_tool_use_on_catalog(
    block_declarations: Sequence[dict],
    axe_source: JavaSource,
    hoe_source: JavaSource,
    shovel_source: JavaSource,
    honeycomb_source: JavaSource,
    weathering_copper_source: JavaSource,
    weathering_copper_blocks_source: JavaSource,
) -> dict[str, list[dict]]:
    """Extract source-owned tool block transforms and client-result predicates."""
    axe_owner = "net.minecraft.world.item.AxeItem"
    strip_field, strip_pairs = _map_put_pairs(
        axe_source, axe_owner, "STRIPPABLES"
    )

    weather_owner = "net.minecraft.world.level.block.WeatheringCopper"
    weather_field, weather_next_pairs = _map_put_pairs(
        weathering_copper_source, weather_owner, "NEXT_BY_BLOCK"
    )
    weather_relations = _weathering_group_relations(
        weathering_copper_blocks_source, "weatheringMapping"
    )
    for group in _group_mapping_calls(
        weathering_copper_source,
        weather_owner,
        weather_field,
        "weatheringMapping",
    ):
        weather_next_pairs.extend(
            _expanded_group_pairs(block_declarations, group, weather_relations)
        )

    honey_owner = "net.minecraft.world.item.HoneycombItem"
    wax_field, wax_pairs = _map_put_pairs(
        honeycomb_source, honey_owner, "WAXABLES"
    )
    wax_relations = _weathering_group_relations(
        weathering_copper_blocks_source, "waxedMapping"
    )
    for group in _group_mapping_calls(
        honeycomb_source, honey_owner, wax_field, "waxedMapping"
    ):
        wax_pairs.extend(
            _expanded_group_pairs(block_declarations, group, wax_relations)
        )

    shovel_owner = "net.minecraft.world.item.ShovelItem"
    _, shovel_pairs = _map_put_pairs(
        shovel_source, shovel_owner, "FLATTENABLES"
    )

    hoe_owner = "net.minecraft.world.item.HoeItem"
    hoe_field = _unique_field(hoe_source, hoe_owner, "TILLABLES")
    map_calls = [
        call
        for call in _calls_within(
            hoe_source, hoe_owner, hoe_field.initializer.span, "of"
        )
        if expression_fact(hoe_source, hoe_source.expression(call.node))
        .get("object", {})
        .get("canonical")
        == "ImmutableMap"
    ]
    if len(map_calls) != 1 or len(map_calls[0].arguments) % 2:
        _fail(hoe_source, hoe_field.initializer, "TILLABLES must contain one even map")
    hoe_entries = []
    for source_expression, pair_expression in zip(
        map_calls[0].arguments[::2],
        map_calls[0].arguments[1::2],
        strict=True,
    ):
        source_block = _block_constant(hoe_source, source_expression)
        pair_fact = expression_fact(hoe_source, pair_expression)
        if (
            source_block is None
            or pair_fact.get("kind") != "method_invocation"
            or pair_fact.get("name") != "of"
            or pair_fact.get("object", {}).get("canonical") != "Pair"
            or len(pair_fact.get("arguments", [])) != 2
        ):
            _fail(hoe_source, pair_expression, "invalid TILLABLES entry")
        predicate, action = pair_fact["arguments"]
        predicate_name = predicate.get("canonical")
        if predicate_name == "HoeItem::onlyIfAirAbove":
            requires_air_above = True
        elif predicate.get("kind") == "lambda_expression" and predicate_name == "context -> true":
            requires_air_above = False
        else:
            _fail(hoe_source, pair_expression, "unsupported TILLABLES predicate")
        if (
            action.get("kind") != "method_invocation"
            or action.get("name")
            not in {"changeIntoState", "changeIntoStateAndDropItem"}
            or not action.get("arguments")
        ):
            _fail(hoe_source, pair_expression, "unsupported TILLABLES action")
        target_fact = action["arguments"][0]
        target = _fact_field(target_fact.get("object", {}), "Blocks")
        if (
            target_fact.get("kind") != "method_invocation"
            or target_fact.get("name") != "defaultBlockState"
            or target is None
        ):
            _fail(hoe_source, pair_expression, "TILLABLES target is not a default block")
        hoe_entries.append(
            {
                "source": source_block,
                "target": target,
                "requires_air_above": requires_air_above,
            }
        )

    validated_hoe = _validate_transform_pairs(
        block_declarations,
        "hoe_tillables",
        [(entry["source"], entry["target"]) for entry in hoe_entries],
    )
    hoe_predicates = {
        entry["source"]: entry["requires_air_above"] for entry in hoe_entries
    }
    for entry in validated_hoe:
        entry["requires_air_above"] = hoe_predicates[entry["source"]]

    return {
        "axe_strippables": _validate_transform_pairs(
            block_declarations, "axe_strippables", strip_pairs
        ),
        "axe_scrapeables": _validate_transform_pairs(
            block_declarations,
            "axe_scrapeables",
            [(target, source) for source, target in weather_next_pairs],
        ),
        "axe_wax_off": _validate_transform_pairs(
            block_declarations,
            "axe_wax_off",
            [(target, source) for source, target in wax_pairs],
        ),
        "honeycomb_waxables": _validate_transform_pairs(
            block_declarations, "honeycomb_waxables", wax_pairs
        ),
        "hoe_tillables": validated_hoe,
        "shovel_flattenables": _validate_transform_pairs(
            block_declarations, "shovel_flattenables", shovel_pairs
        ),
    }


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


def _float(source: JavaSource, expression: JavaExpression) -> float:
    if expression.node.type != "decimal_floating_point_literal":
        _fail(source, expression, "expected a Java decimal floating-point literal")
    spelling = expression.canonical.replace("_", "")
    if spelling[-1:] in {"f", "F", "d", "D"}:
        spelling = spelling[:-1]
    try:
        value = float(spelling)
    except ValueError:
        _fail(source, expression, "invalid Java floating-point literal")
    if not math.isfinite(value):
        _fail(source, expression, "expected a finite Java floating-point literal")
    return struct.unpack(">f", struct.pack(">f", value))[0]


def _f32(value: float) -> float:
    return struct.unpack(">f", struct.pack(">f", value))[0]


def _f32_bits(value: float) -> int:
    return struct.unpack(">I", struct.pack(">f", value))[0]


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


def _food_properties(foods_source: JavaSource) -> dict[str, dict[str, Any]]:
    stew_methods = [
        method
        for method in foods_source.methods_for(FOODS_OWNER)
        if method.name == "stew" and method.body is not None
    ]
    if len(stew_methods) != 1:
        raise ItemRegistryCatalogExtractionError(
            f"{foods_source.relative_path}: expected exactly one Foods.stew helper"
        )
    stew_method = stew_methods[0]
    stew_modifier_calls = [
        call
        for call in foods_source.calls_for(FOODS_OWNER)
        if call.name == "saturationModifier"
        and stew_method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= stew_method.body.end_byte
    ]
    if len(stew_modifier_calls) != 1 or len(stew_modifier_calls[0].arguments) != 1:
        raise ItemRegistryCatalogExtractionError(
            f"{foods_source.relative_path}: Foods.stew must set one saturation modifier"
        )
    stew_modifier = _float(foods_source, stew_modifier_calls[0].arguments[0])

    properties: dict[str, dict[str, Any]] = {}
    for field in foods_source.fields_for(FOODS_OWNER):
        if field.type_spelling != "FoodProperties":
            continue
        if field.initializer is None:
            raise ItemRegistryCatalogExtractionError(
                f"{foods_source.relative_path}: Foods.{field.name} lacks initializer"
            )
        if not {"public", "static", "final"}.issubset(field.modifiers):
            raise ItemRegistryCatalogExtractionError(
                f"{foods_source.relative_path}: Foods.{field.name} must be public static final"
            )
        calls = [
            call
            for call in foods_source.calls_for(FOODS_OWNER)
            if field.initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= field.initializer.span.end_byte
        ]
        nutrition_calls = [call for call in calls if call.name == "nutrition"]
        stew_calls = [call for call in calls if call.name == "stew"]
        if len(nutrition_calls) == 1 and not stew_calls:
            if len(nutrition_calls[0].arguments) != 1:
                _fail(
                    foods_source,
                    field.initializer,
                    "nutrition must have one argument",
                )
            nutrition = _integer(foods_source, nutrition_calls[0].arguments[0])
        elif len(stew_calls) == 1 and not nutrition_calls:
            if len(stew_calls[0].arguments) != 1:
                _fail(foods_source, field.initializer, "stew must have one argument")
            nutrition = _integer(foods_source, stew_calls[0].arguments[0])
        else:
            _fail(
                foods_source,
                field.initializer,
                "expected exactly one nutrition or stew initializer",
            )
        modifier_calls = [call for call in calls if call.name == "saturationModifier"]
        if len(modifier_calls) == 1 and not stew_calls:
            if len(modifier_calls[0].arguments) != 1:
                _fail(
                    foods_source,
                    field.initializer,
                    "saturationModifier must have one argument",
                )
            modifier = _float(foods_source, modifier_calls[0].arguments[0])
        elif not modifier_calls and len(stew_calls) == 1:
            modifier = stew_modifier
        else:
            _fail(
                foods_source,
                field.initializer,
                "expected exactly one saturation modifier source",
            )
        always_edible = any(call.name == "alwaysEdible" for call in calls)
        saturation = _f32(_f32(_f32(float(nutrition)) * modifier) * _f32(2.0))
        properties[field.name] = {
            "nutrition": nutrition,
            "saturation_bits": _f32_bits(saturation),
            "can_always_eat": always_edible,
        }
    if not properties:
        raise ItemRegistryCatalogExtractionError(
            f"{foods_source.relative_path}: no FoodProperties constants found"
        )
    return properties


@dataclass(frozen=True)
class ExtractedItemRegistryCatalog:
    item_id_constants: dict[str, str]
    item_constants: dict[str, int]
    food_properties: dict[str, dict[str, Any]]
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
                "net/minecraft/world/food/Foods.java",
            ],
            "item_id_constants": self.item_id_constants,
            "food_properties": self.food_properties,
            "declarations": list(self.declarations),
        }


def extract_item_registry_catalog(
    items_source: JavaSource,
    item_ids_source: JavaSource,
    item_source: JavaSource,
    foods_source: JavaSource,
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
    return ExtractedItemRegistryCatalog(
        constants,
        item_constants,
        _food_properties(foods_source),
        tuple(declarations),
    )
