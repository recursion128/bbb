"""Tree-sitter-backed targeted block-use registration extraction.

This module derives the small item-driven registration tables used by
``ComposterBlock`` and ``CauldronInteractions``. Runtime predicates and state
transitions remain owned by the Rust world implementation.
"""

from __future__ import annotations

import json
import struct
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaSource, expression_fact


class TargetedBlockUseCatalogExtractionError(ValueError):
    pass


COMPOSTER_OWNER = "net.minecraft.world.level.block.ComposterBlock"
CAULDRON_OWNER = "net.minecraft.core.cauldron.CauldronInteractions"
ITEM_TAGS_OWNER = "net.minecraft.tags.ItemTags"
CAULDRON_DISPATCHERS = ("EMPTY", "WATER", "LAVA", "POWDER_SNOW")


@dataclass(frozen=True)
class CompostableEntry:
    item_field: str
    chance_bits: int


@dataclass(frozen=True)
class CauldronItemInteractionEntry:
    dispatcher: str
    item_field: str
    handler: str


@dataclass(frozen=True)
class CauldronTagInteractionEntry:
    dispatcher: str
    tag_resource_id: str
    handler: str


@dataclass(frozen=True)
class TargetedBlockUseCatalog:
    compostables: tuple[CompostableEntry, ...]
    cauldron_items: tuple[CauldronItemInteractionEntry, ...]
    cauldron_tags: tuple[CauldronTagInteractionEntry, ...]


def _fail(source: JavaSource, expression, message: str):
    line = expression.span.start_point[0] + 1
    column = expression.span.start_point[1] + 1
    raise TargetedBlockUseCatalogExtractionError(
        f"{source.relative_path}:{line}:{column}: {message}"
    )


def _method(source: JavaSource, owner: str, name: str):
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == name and method.body is not None
    ]
    if len(methods) != 1:
        raise TargetedBlockUseCatalogExtractionError(
            f"{source.relative_path}: expected exactly one {owner}.{name} method"
        )
    return methods[0]


def _calls_within(source: JavaSource, owner: str, method_name: str):
    method = _method(source, owner, method_name)
    return [
        call
        for call in source.calls_for(owner)
        if method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]


def _field_access(
    source: JavaSource, expression: JavaExpression, expected_owner: str
) -> str | None:
    fact = expression_fact(source, expression)
    if fact.get("kind") != "field_access":
        return None
    object_fact = fact.get("object")
    if object_fact is None or object_fact.get("canonical") != expected_owner:
        return None
    name = fact.get("name")
    return name if isinstance(name, str) and name else None


def _walk_facts(fact: dict):
    yield fact
    object_fact = fact.get("object")
    if object_fact is not None:
        yield from _walk_facts(object_fact)
    for key in ("arguments", "children"):
        for child in fact.get(key, []):
            yield from _walk_facts(child)


def _float32_bits(source: JavaSource, expression: JavaExpression) -> int:
    fact = expression_fact(source, expression)
    if fact.get("kind") != "decimal_floating_point_literal":
        _fail(source, expression, "compost chance must be a floating-point literal")
    spelling = fact["canonical"]
    if not spelling.endswith(("F", "f")):
        _fail(source, expression, "compost chance must use Java float precision")
    try:
        value = float(spelling[:-1].replace("_", ""))
    except ValueError:
        _fail(source, expression, f"invalid compost chance {spelling!r}")
    bits = struct.unpack(">I", struct.pack(">f", value))[0]
    if not 0 < value <= 1:
        _fail(source, expression, f"compost chance must be in (0, 1], got {value}")
    return bits


def extract_compostables(source: JavaSource) -> tuple[CompostableEntry, ...]:
    entries = []
    seen = set()
    for call in _calls_within(source, COMPOSTER_OWNER, "bootStrap"):
        if call.name != "add" or call.qualifier is not None:
            continue
        if len(call.arguments) != 2:
            _fail(source, call, "ComposterBlock.add must have two arguments")
        item_field = _field_access(source, call.arguments[1], "Items")
        if item_field is None:
            _fail(source, call.arguments[1], "compostable must be an Items constant")
        if item_field in seen:
            _fail(source, call.arguments[1], f"duplicate compostable Items.{item_field}")
        seen.add(item_field)
        entries.append(
            CompostableEntry(item_field, _float32_bits(source, call.arguments[0]))
        )
    if not entries:
        raise TargetedBlockUseCatalogExtractionError(
            f"{source.relative_path}: no compostable registrations"
        )
    return tuple(entries)


def _method_reference_handler(source: JavaSource, expression: JavaExpression) -> str:
    fact = expression_fact(source, expression)
    if fact.get("kind") != "method_reference":
        _fail(source, expression, "cauldron handler must be a method reference")
    handlers = {
        "CauldronInteractions::fillLavaInteraction": "PourLavaBucket",
        "CauldronInteractions::fillWaterInteraction": "PourWaterBucket",
        "CauldronInteractions::fillPowderSnowInteraction": "PourPowderSnowBucket",
        "CauldronInteractions::dyedItemIteration": "CleanDyedItem",
        "CauldronInteractions::bannerInteraction": "CleanBanner",
        "CauldronInteractions::shulkerBoxInteraction": "CleanShulkerBox",
    }
    handler = handlers.get(fact["canonical"])
    if handler is None:
        _fail(source, expression, f"unsupported cauldron handler {fact['canonical']!r}")
    return handler


def _lambda_handler(
    source: JavaSource,
    dispatcher: str,
    item_field: str,
    expression: JavaExpression,
) -> str:
    fact = expression_fact(source, expression)
    if fact.get("kind") != "lambda_expression":
        _fail(source, expression, "special cauldron handler must be a lambda")
    handlers = {
        ("EMPTY", "POTION"): "PourWaterPotionIntoEmpty",
        ("WATER", "BUCKET"): "TakeWaterBucket",
        ("WATER", "GLASS_BOTTLE"): "TakeWaterBottle",
        ("WATER", "POTION"): "PourWaterPotionIntoWater",
        ("LAVA", "BUCKET"): "TakeLavaBucket",
        ("POWDER_SNOW", "BUCKET"): "TakePowderSnowBucket",
    }
    handler = handlers.get((dispatcher, item_field))
    if handler is None:
        _fail(
            source,
            expression,
            f"unsupported {dispatcher}/Items.{item_field} cauldron lambda",
        )

    facts = tuple(_walk_facts(fact))
    calls = {
        entry.get("name")
        for entry in facts
        if entry.get("kind") == "method_invocation"
    }
    fields = {
        entry["canonical"]
        for entry in facts
        if entry.get("kind") == "field_access"
    }
    required_calls = {
        "PourWaterPotionIntoEmpty": {"get", "is", "isClientSide"},
        "TakeWaterBucket": {"fillBucket", "getValue"},
        "TakeWaterBottle": {"isClientSide", "lowerFillLevel"},
        "PourWaterPotionIntoWater": {"getValue", "get", "is", "isClientSide"},
        "TakeLavaBucket": {"fillBucket"},
        "TakePowderSnowBucket": {"fillBucket", "getValue"},
    }[handler]
    required_fields = {
        "PourWaterPotionIntoEmpty": {
            "DataComponents.POTION_CONTENTS",
            "Potions.WATER",
            "InteractionResult.SUCCESS",
            "InteractionResult.TRY_WITH_EMPTY_HAND",
        },
        "TakeWaterBucket": {
            "Items.WATER_BUCKET",
            "LayeredCauldronBlock.LEVEL",
        },
        "TakeWaterBottle": {
            "Items.POTION",
            "Potions.WATER",
            "InteractionResult.SUCCESS",
        },
        "PourWaterPotionIntoWater": {
            "DataComponents.POTION_CONTENTS",
            "Potions.WATER",
            "LayeredCauldronBlock.LEVEL",
            "InteractionResult.SUCCESS",
            "InteractionResult.TRY_WITH_EMPTY_HAND",
        },
        "TakeLavaBucket": {
            "Items.LAVA_BUCKET",
        },
        "TakePowderSnowBucket": {
            "Items.POWDER_SNOW_BUCKET",
            "LayeredCauldronBlock.LEVEL",
        },
    }[handler]
    missing_calls = required_calls - calls
    missing_fields = required_fields - fields
    if missing_calls or missing_fields:
        _fail(
            source,
            expression,
            f"{handler} lambda lost semantic markers: "
            f"calls={sorted(missing_calls)}, fields={sorted(missing_fields)}",
        )
    return handler


def _item_tag_resource_ids(source: JavaSource) -> dict[str, str]:
    result = {}
    for field in source.fields_for(ITEM_TAGS_OWNER):
        if field.initializer is None:
            continue
        fact = expression_fact(source, field.initializer)
        if fact.get("kind") != "method_invocation" or fact.get("name") != "bind":
            continue
        arguments = fact.get("arguments", [])
        if len(arguments) != 1 or arguments[0].get("kind") != "string_literal":
            _fail(
                source,
                field.initializer,
                f"ItemTags.{field.name} bind must use one string",
            )
        value = json.loads(arguments[0]["canonical"])
        if not isinstance(value, str):
            _fail(
                source,
                field.initializer,
                f"ItemTags.{field.name} bind is not a string",
            )
        result[field.name] = "minecraft:" + value
    return result


def extract_cauldron_interactions(
    source: JavaSource, item_tags_source: JavaSource
) -> tuple[
    tuple[CauldronItemInteractionEntry, ...],
    tuple[CauldronTagInteractionEntry, ...],
]:
    defaults = []
    for call in _calls_within(source, CAULDRON_OWNER, "addDefaultInteractions"):
        if (
            call.name != "put"
            or call.qualifier is None
            or call.qualifier.canonical != "interactionMap"
        ):
            continue
        if len(call.arguments) != 2:
            _fail(source, call, "default cauldron put must have two arguments")
        item_field = _field_access(source, call.arguments[0], "Items")
        if item_field is None:
            _fail(
                source,
                call.arguments[0],
                "default cauldron key must be an Items constant",
            )
        defaults.append(
            (item_field, _method_reference_handler(source, call.arguments[1]))
        )
    if not defaults:
        raise TargetedBlockUseCatalogExtractionError(
            f"{source.relative_path}: no default cauldron interactions"
        )

    item_entries: dict[tuple[str, str], str] = {}
    tag_entries: dict[tuple[str, str], str] = {}
    for call in _calls_within(source, CAULDRON_OWNER, "bootStrap"):
        if call.name == "addDefaultInteractions" and call.qualifier is None:
            if len(call.arguments) != 1:
                _fail(source, call, "addDefaultInteractions must have one argument")
            dispatcher = call.arguments[0].canonical
            if dispatcher not in CAULDRON_DISPATCHERS:
                _fail(
                    source,
                    call.arguments[0],
                    f"unknown cauldron dispatcher {dispatcher!r}",
                )
            for item_field, handler in defaults:
                item_entries[(dispatcher, item_field)] = handler
            continue
        if (
            call.name != "put"
            or call.qualifier is None
            or call.qualifier.canonical not in CAULDRON_DISPATCHERS
        ):
            continue
        if len(call.arguments) != 2:
            _fail(source, call, "cauldron put must have two arguments")
        dispatcher = call.qualifier.canonical
        item_field = _field_access(source, call.arguments[0], "Items")
        if item_field is not None:
            fact = expression_fact(source, call.arguments[1])
            handler = (
                _method_reference_handler(source, call.arguments[1])
                if fact.get("kind") == "method_reference"
                else _lambda_handler(source, dispatcher, item_field, call.arguments[1])
            )
            item_entries[(dispatcher, item_field)] = handler
            continue
        tag_field = _field_access(source, call.arguments[0], "ItemTags")
        if tag_field is None:
            _fail(source, call.arguments[0], "cauldron key must be Items or ItemTags")
        tag_entries[(dispatcher, tag_field)] = _method_reference_handler(
            source, call.arguments[1]
        )

    tag_resources = _item_tag_resource_ids(item_tags_source)
    resolved_tags = []
    for (dispatcher, tag_field), handler in tag_entries.items():
        resource_id = tag_resources.get(tag_field)
        if resource_id is None:
            raise TargetedBlockUseCatalogExtractionError(
                f"{item_tags_source.relative_path}: unresolved ItemTags.{tag_field}"
            )
        resolved_tags.append(
            CauldronTagInteractionEntry(dispatcher, resource_id, handler)
        )
    return (
        tuple(
            CauldronItemInteractionEntry(dispatcher, item_field, handler)
            for (dispatcher, item_field), handler in item_entries.items()
        ),
        tuple(resolved_tags),
    )


def extract_targeted_block_use_catalog(
    composter_source: JavaSource,
    cauldron_source: JavaSource,
    item_tags_source: JavaSource,
) -> TargetedBlockUseCatalog:
    cauldron_items, cauldron_tags = extract_cauldron_interactions(
        cauldron_source, item_tags_source
    )
    return TargetedBlockUseCatalog(
        extract_compostables(composter_source),
        cauldron_items,
        cauldron_tags,
    )
