"""Tree-sitter-backed Vanilla creative-mode tab catalog extraction."""

from __future__ import annotations

import json
from dataclasses import dataclass

from tree_sitter import Node

from item_registry_catalog_extractor import (
    extract_item_declarations,
    extract_item_id_constants,
    item_resource_ids_for_declaration,
)
from vanilla_java_source import (
    JavaCall,
    JavaExpression,
    JavaSource,
    expression_fact,
)


CREATIVE_TABS_OWNER = "net.minecraft.world.item.CreativeModeTabs"
CREATIVE_TAB_OWNER = "net.minecraft.world.item.CreativeModeTab"
CREATIVE_TAB_BUILDER_OWNER = "net.minecraft.world.item.CreativeModeTab.Builder"
CREATIVE_TAB_OUTPUT_OWNER = "net.minecraft.world.item.CreativeModeTab.Output"

VISIBILITY_BY_CANONICAL = {
    "CreativeModeTab.TabVisibility.PARENT_AND_SEARCH_TABS": "parent_and_search",
    "CreativeModeTab.TabVisibility.PARENT_TAB_ONLY": "parent_only",
    "CreativeModeTab.TabVisibility.SEARCH_TAB_ONLY": "search_only",
}
WEATHERING_ACCESSOR_INDEX = {
    "unaffected": 0,
    "exposed": 1,
    "weathered": 2,
    "oxidized": 3,
    "waxed": 4,
    "waxedExposed": 5,
    "waxedWeathered": 6,
    "waxedOxidized": 7,
}
DYNAMIC_HELPERS = {
    "generateEnchantmentBookTypesAllLevels",
    "generateEnchantmentBookTypesOnlyMaxLevel",
    "generateFireworksAllDurations",
    "generateInstrumentTypes",
    "generateOminousBottles",
    "generatePotionEffectTypes",
    "generatePresetPaintings",
    "generateSuspiciousStews",
}


class CreativeModeTabCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class CreativeModeTabEntry:
    kind: str
    visibility: str
    item_resource_id: str | None = None
    tag_resource_id: str | None = None
    painting_placeable: bool | None = None


@dataclass(frozen=True)
class CreativeModeTabDefinition:
    resource_id: str
    row: str
    column: int
    title_key: str
    icon_item_resource_id: str
    tab_type: str
    background: str
    aligned_right: bool
    show_title: bool
    can_scroll: bool
    requires_permissions: bool
    entries: tuple[CreativeModeTabEntry, ...]


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise CreativeModeTabCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _compact(value: str) -> str:
    return "".join(value.split())


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected string literal")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "expected non-empty string literal")
    return value


def _field_access(
    source: JavaSource, expression: JavaExpression, expected_owner: str
) -> str:
    if expression.node.type != "field_access":
        _fail(source, expression, f"expected {expected_owner} constant")
    owner = expression.node.child_by_field_name("object")
    field = expression.node.child_by_field_name("field")
    if owner is None or field is None or _compact(source.text(owner)) != expected_owner:
        _fail(source, expression, f"expected {expected_owner} constant")
    return source.text(field)


def _walk_facts(fact: dict):
    yield fact
    object_fact = fact.get("object")
    if object_fact is not None:
        yield from _walk_facts(object_fact)
    for key in ("arguments", "children"):
        for child in fact.get(key, []):
            yield from _walk_facts(child)


def _calls_within(
    source: JavaSource, start_byte: int, end_byte: int
) -> list[JavaCall]:
    calls = [
        call
        for call in source.calls
        if start_byte <= call.span.start_byte and call.span.end_byte <= end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    return calls


def _method_call(
    source: JavaSource,
    expression: JavaExpression,
    expected_name: str,
    expected_arity: int,
) -> tuple[dict, list[dict]]:
    fact = expression_fact(source, expression)
    arguments = fact.get("arguments", [])
    if (
        fact.get("kind") != "method_invocation"
        or fact.get("name") != expected_name
        or len(arguments) != expected_arity
    ):
        _fail(
            source,
            expression,
            f"expected {expected_name} invocation with {expected_arity} arguments",
        )
    return fact, arguments


def _item_resources(
    items_source: JavaSource, item_ids_source: JavaSource
) -> dict[str, tuple[str, ...]]:
    constants = extract_item_id_constants(item_ids_source)
    result: dict[str, tuple[str, ...]] = {}
    for declaration in extract_item_declarations(items_source):
        resources = tuple(
            "minecraft:" + resource_id
            for resource_id in item_resource_ids_for_declaration(
                declaration, constants
            )
        )
        field = declaration["field"]
        if field in result:
            raise CreativeModeTabCatalogExtractionError(
                f"{items_source.relative_path}: duplicate Items.{field}"
            )
        result[field] = resources
    return result


def _one_item_resource(
    source: JavaSource,
    expression: JavaExpression,
    item_resources: dict[str, tuple[str, ...]],
    owner: str = "Items",
) -> str:
    field = _field_access(source, expression, owner)
    resources = item_resources.get(field)
    if resources is None:
        _fail(source, expression, f"{owner}.{field} is not a registered item")
    if len(resources) != 1:
        _fail(source, expression, f"{owner}.{field} names an item family")
    return resources[0]


def _validate_creative_tab_defaults(source: JavaSource) -> str:
    fields = {
        field.name: field
        for field in source.fields_for(CREATIVE_TAB_OWNER)
    }
    default_background = fields.get("DEFAULT_BACKGROUND")
    if (
        default_background is None
        or default_background.initializer is None
        or _compact(default_background.initializer.canonical)
        != 'createTextureLocation("items")'
    ):
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}: CreativeModeTab.DEFAULT_BACKGROUND drifted"
        )

    expected_defaults = {
        "canScroll": "true",
        "showTitle": "true",
        "alignedRight": "false",
        "type": "CreativeModeTab.Type.CATEGORY",
        "backgroundTexture": "CreativeModeTab.DEFAULT_BACKGROUND",
    }
    builder_fields = {
        field.name: field
        for field in source.fields_for(CREATIVE_TAB_BUILDER_OWNER)
    }
    for name, expected in expected_defaults.items():
        field = builder_fields.get(name)
        if (
            field is None
            or field.initializer is None
            or _compact(field.initializer.canonical) != _compact(expected)
        ):
            raise CreativeModeTabCatalogExtractionError(
                f"{source.relative_path}: CreativeModeTab.Builder.{name} default drifted"
            )

    accept_defaults = [
        method
        for method in source.methods_for(CREATIVE_TAB_OUTPUT_OWNER)
        if method.name == "accept"
        and method.body is not None
        and len(method.parameters) == 1
        and "ItemLike" in method.parameters[0].canonical
    ]
    if len(accept_defaults) != 1:
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}: expected one Output.accept(ItemLike) default"
        )
    body = _compact(
        accept_defaults[0].body.bytes(source.source).decode("utf-8")
    )
    expected_body = _compact(
        "{this.accept(new ItemStack(item),"
        "CreativeModeTab.TabVisibility.PARENT_AND_SEARCH_TABS);}"
    )
    if body != expected_body:
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}: Output.accept(ItemLike) default drifted"
        )
    return "items"


def _tab_keys(source: JavaSource) -> dict[str, str]:
    keys: dict[str, str] = {}
    resource_ids: set[str] = set()
    for field in source.fields_for(CREATIVE_TABS_OWNER):
        if _compact(field.type_spelling) != "ResourceKey<CreativeModeTab>":
            continue
        if field.initializer is None:
            raise CreativeModeTabCatalogExtractionError(
                f"{source.relative_path}: {field.name} tab key lacks initializer"
            )
        _method_call(source, field.initializer, "createKey", 1)
        argument_nodes = field.initializer.node.child_by_field_name("arguments")
        if argument_nodes is None:
            _fail(source, field.initializer, "creative tab key lacks arguments")
        resource_id = "minecraft:" + _string(
            source, source.expression(argument_nodes.named_children[0])
        )
        if resource_id in resource_ids:
            _fail(source, field.initializer, f"duplicate creative tab key {resource_id}")
        resource_ids.add(resource_id)
        keys[field.name] = resource_id
    if not keys:
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}: no creative tab keys found"
        )
    return keys


def _texture_fields(source: JavaSource) -> dict[str, str]:
    textures: dict[str, str] = {}
    for field in source.fields_for(CREATIVE_TABS_OWNER):
        if field.type_spelling != "Identifier" or field.initializer is None:
            continue
        if field.initializer.node.type != "method_invocation":
            continue
        name = field.initializer.node.child_by_field_name("name")
        arguments = field.initializer.node.child_by_field_name("arguments")
        if (
            name is None
            or source.text(name) != "createTextureLocation"
            or arguments is None
            or len(arguments.named_children) != 1
        ):
            continue
        textures[field.name] = _string(
            source, source.expression(arguments.named_children[0])
        )
    return textures


def _fluent_builder_facts(source: JavaSource, expression: JavaExpression) -> list[dict]:
    fact = expression_fact(source, expression)
    calls: list[dict] = []
    while fact.get("kind") == "method_invocation":
        calls.append(fact)
        fact = fact.get("object")
        if fact is None:
            break
    if fact is None or fact.get("canonical") != "CreativeModeTab":
        _fail(source, expression, "creative tab builder is not rooted at CreativeModeTab")
    calls.reverse()
    names = [call.get("name") for call in calls]
    if not names or names[0] != "builder" or names[-1] != "build":
        _fail(source, expression, "creative tab builder lacks builder/build boundary")
    allowed = {
        "alignedRight",
        "backgroundTexture",
        "build",
        "builder",
        "displayItems",
        "hideTitle",
        "icon",
        "noScrollBar",
        "title",
        "type",
    }
    unknown = [name for name in names if name not in allowed]
    if unknown:
        _fail(source, expression, f"unsupported creative tab builder calls {unknown}")
    for name in allowed - {"alignedRight", "backgroundTexture", "displayItems", "hideTitle", "noScrollBar", "type"}:
        if names.count(name) != 1:
            _fail(source, expression, f"expected one creative tab builder {name} call")
    for name in allowed:
        if names.count(name) > 1:
            _fail(source, expression, f"duplicate creative tab builder {name} call")
    return calls


def _fact_field(fact: dict, owner: str) -> str | None:
    if fact.get("kind") != "field_access":
        return None
    object_fact = fact.get("object")
    if object_fact is None or _compact(object_fact.get("canonical", "")) != owner:
        return None
    name = fact.get("name")
    return name if isinstance(name, str) else None


def _fact_string(fact: dict) -> str | None:
    if fact.get("kind") != "string_literal":
        return None
    try:
        value = json.loads(fact.get("canonical", ""))
    except (json.JSONDecodeError, TypeError):
        return None
    return value if isinstance(value, str) and value else None


def _builder_metadata(
    source: JavaSource,
    expression: JavaExpression,
    item_resources: dict[str, tuple[str, ...]],
    textures: dict[str, str],
    default_background: str,
) -> tuple[dict, JavaCall | None]:
    calls = _fluent_builder_facts(source, expression)
    by_name = {call["name"]: call for call in calls}

    builder_arguments = by_name["builder"].get("arguments", [])
    if len(builder_arguments) != 2:
        _fail(source, expression, "creative tab builder requires row and column")
    row = _fact_field(builder_arguments[0], "CreativeModeTab.Row")
    if row not in {"TOP", "BOTTOM"}:
        _fail(source, expression, "unsupported creative tab row")
    column_spelling = builder_arguments[1].get("canonical", "")
    try:
        column = int(column_spelling)
    except ValueError:
        _fail(source, expression, "creative tab column must be a decimal literal")
    if not 0 <= column <= 6:
        _fail(source, expression, "creative tab column must be between zero and six")

    title_arguments = by_name["title"].get("arguments", [])
    if len(title_arguments) != 1:
        _fail(source, expression, "creative tab title requires one argument")
    title_fact = title_arguments[0]
    if (
        title_fact.get("kind") != "method_invocation"
        or title_fact.get("name") != "translatable"
        or title_fact.get("object", {}).get("canonical") != "Component"
        or len(title_fact.get("arguments", [])) != 1
    ):
        _fail(source, expression, "creative tab title is not Component.translatable")
    title_key = _fact_string(title_fact["arguments"][0])
    if title_key is None:
        _fail(source, expression, "creative tab title key is not a string")

    icon_arguments = by_name["icon"].get("arguments", [])
    if len(icon_arguments) != 1:
        _fail(source, expression, "creative tab icon requires one supplier")
    creations = [
        fact
        for fact in _walk_facts(icon_arguments[0])
        if fact.get("kind") == "object_creation_expression"
        and fact.get("name") == "ItemStack"
    ]
    if len(creations) != 1 or len(creations[0].get("arguments", [])) != 1:
        _fail(source, expression, "creative tab icon must create one ItemStack")
    icon_fact = creations[0]["arguments"][0]
    icon_field = _fact_field(icon_fact, "Items") or _fact_field(icon_fact, "Blocks")
    icon_values = item_resources.get(icon_field or "")
    if icon_values is None or len(icon_values) != 1:
        _fail(source, expression, "creative tab icon does not resolve to one item")

    tab_type = "category"
    if "type" in by_name:
        arguments = by_name["type"].get("arguments", [])
        type_name = (
            _fact_field(arguments[0], "CreativeModeTab.Type")
            if len(arguments) == 1
            else None
        )
        if type_name not in {"CATEGORY", "HOTBAR", "INVENTORY", "SEARCH"}:
            _fail(source, expression, "unsupported creative tab type")
        tab_type = type_name.lower()

    background = default_background
    if "backgroundTexture" in by_name:
        arguments = by_name["backgroundTexture"].get("arguments", [])
        name = arguments[0].get("canonical") if len(arguments) == 1 else None
        background = textures.get(name or "")
        if background is None:
            _fail(source, expression, "unresolved creative tab background texture")
    if background not in {"items", "item_search", "inventory"}:
        _fail(source, expression, f"unsupported creative tab background {background}")

    display_calls = [
        call
        for call in _calls_within(
            source, expression.span.start_byte, expression.span.end_byte
        )
        if call.name == "displayItems"
    ]
    if len(display_calls) > 1:
        _fail(source, expression, "creative tab has multiple displayItems calls")
    return (
        {
            "row": row.lower(),
            "column": column,
            "title_key": title_key,
            "icon_item_resource_id": icon_values[0],
            "tab_type": tab_type,
            "background": background,
            "aligned_right": "alignedRight" in by_name,
            "show_title": "hideTitle" not in by_name,
            "can_scroll": "noScrollBar" not in by_name,
        },
        display_calls[0] if display_calls else None,
    )


def _visibility(source: JavaSource, expression: JavaExpression) -> str:
    visibility = VISIBILITY_BY_CANONICAL.get(expression.canonical)
    if visibility is None:
        _fail(source, expression, "unsupported creative tab visibility")
    return visibility


def _weathering_item(
    source: JavaSource,
    expression: JavaExpression,
    item_resources: dict[str, tuple[str, ...]],
) -> str | None:
    fact = expression_fact(source, expression)
    if fact.get("kind") != "method_invocation" or fact.get("arguments"):
        return None
    accessor = fact.get("name")
    index = WEATHERING_ACCESSOR_INDEX.get(accessor)
    field = _fact_field(fact.get("object", {}), "Items")
    if index is None or field is None:
        return None
    resources = item_resources.get(field)
    if resources is None or len(resources) != len(WEATHERING_ACCESSOR_INDEX):
        _fail(source, expression, f"Items.{field} is not a weathering item family")
    return resources[index]


def _accept_entry(
    source: JavaSource,
    call: JavaCall,
    item_resources: dict[str, tuple[str, ...]],
) -> CreativeModeTabEntry:
    if len(call.arguments) not in {1, 2}:
        _fail(source, source.expression(call.node), "unsupported output.accept arity")
    visibility = (
        _visibility(source, call.arguments[1])
        if len(call.arguments) == 2
        else "parent_and_search"
    )
    item = call.arguments[0]
    if item.node.type == "field_access":
        return CreativeModeTabEntry(
            "item",
            visibility,
            _one_item_resource(source, item, item_resources),
        )
    weathering = _weathering_item(source, item, item_resources)
    if weathering is not None:
        return CreativeModeTabEntry("item", visibility, weathering)

    compact = _compact(item.canonical)
    if compact == _compact(
        "Raid.getOminousBannerInstance("
        "parameters.holders().lookupOrThrow(Registries.BANNER_PATTERN))"
    ):
        return CreativeModeTabEntry("ominous_banner", visibility)
    if compact == _compact(
        "TestBlock.setModeOnStack(new ItemStack(Items.TEST_BLOCK), mode)"
    ):
        return CreativeModeTabEntry("test_block_modes", visibility)
    if compact == _compact(
        "LightBlock.setLightOnStack(new ItemStack(Items.LIGHT), lightLevel)"
    ):
        return CreativeModeTabEntry("light_levels", visibility)
    _fail(source, item, "unsupported creative output.accept value")


def _helper_entry(
    source: JavaSource,
    call: JavaCall,
    output_name: str,
    item_resources: dict[str, tuple[str, ...]],
) -> CreativeModeTabEntry:
    arguments = call.arguments
    if not arguments or arguments[0].canonical != output_name:
        _fail(source, source.expression(call.node), "creative helper output drifted")

    if call.name == "generateFireworksAllDurations" and len(arguments) == 2:
        return CreativeModeTabEntry(
            "fireworks", _visibility(source, arguments[1])
        )
    if call.name == "generateSuspiciousStews" and len(arguments) == 2:
        return CreativeModeTabEntry(
            "suspicious_stews", _visibility(source, arguments[1])
        )
    if call.name == "generateOminousBottles" and len(arguments) == 2:
        return CreativeModeTabEntry(
            "ominous_bottles", _visibility(source, arguments[1])
        )
    if (
        call.name == "generateEnchantmentBookTypesOnlyMaxLevel"
        and len(arguments) == 3
        and arguments[1].canonical == "enchantments"
    ):
        return CreativeModeTabEntry(
            "enchantment_books_max", _visibility(source, arguments[2])
        )
    if (
        call.name == "generateEnchantmentBookTypesAllLevels"
        and len(arguments) == 3
        and arguments[1].canonical == "enchantments"
    ):
        return CreativeModeTabEntry(
            "enchantment_books_all", _visibility(source, arguments[2])
        )
    if (
        call.name == "generatePotionEffectTypes"
        and len(arguments) == 5
        and arguments[1].canonical == "potions"
        and _compact(arguments[4].canonical) == "parameters.enabledFeatures()"
    ):
        return CreativeModeTabEntry(
            "potions",
            _visibility(source, arguments[3]),
            _one_item_resource(source, arguments[2], item_resources),
        )
    if (
        call.name == "generateInstrumentTypes"
        and len(arguments) == 5
        and arguments[1].canonical == "instruments"
    ):
        tag = _field_access(source, arguments[3], "InstrumentTags")
        return CreativeModeTabEntry(
            "instruments",
            _visibility(source, arguments[4]),
            _one_item_resource(source, arguments[2], item_resources),
            "minecraft:" + tag.lower(),
        )
    if (
        call.name == "generatePresetPaintings"
        and len(arguments) == 5
        and _compact(arguments[1].canonical) == "parameters.holders()"
        and "paintings" in arguments[2].canonical
    ):
        predicate = _compact(arguments[3].canonical)
        if predicate == "variant->variant.is(PaintingVariantTags.PLACEABLE)":
            placeable = True
        elif predicate == "variant->!variant.is(PaintingVariantTags.PLACEABLE)":
            placeable = False
        else:
            _fail(source, arguments[3], "unsupported preset painting predicate")
        return CreativeModeTabEntry(
            "preset_paintings",
            _visibility(source, arguments[4]),
            painting_placeable=placeable,
        )
    _fail(source, source.expression(call.node), f"unsupported {call.name} call")


def _lambda_parts(
    source: JavaSource, expression: JavaExpression
) -> tuple[str, str, Node]:
    if expression.node.type != "lambda_expression":
        _fail(source, expression, "displayItems argument must be a lambda")
    parameters = expression.node.child_by_field_name("parameters")
    body = expression.node.child_by_field_name("body")
    if parameters is None or body is None or body.type != "block":
        _fail(source, expression, "displayItems lambda must have a block body")
    names = [
        source.text(child)
        for child in parameters.named_children
        if child.type == "identifier"
    ]
    if len(names) != 2:
        _fail(source, expression, "displayItems lambda must have two inferred parameters")
    return names[0], names[1], body


def _event_permission_gate(
    source: JavaSource,
    body: Node,
    events: list[JavaCall],
    parameters_name: str,
) -> bool:
    gated: dict[tuple[int, int], Node] = {}
    for event in events:
        current = event.node.parent
        while current is not None and current != body:
            if current.type == "if_statement":
                gated[(current.start_byte, current.end_byte)] = current
            current = current.parent
    if not gated:
        return False
    if len(gated) != 1:
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}: unsupported nested creative item condition"
        )
    gate = next(iter(gated.values()))
    condition = gate.child_by_field_name("condition")
    if condition is None or _compact(source.text(condition)) != _compact(
        f"({parameters_name}.hasPermissions())"
    ):
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}: unsupported creative item condition "
            f"{source.text(condition) if condition is not None else '<missing>'}"
        )
    if any(
        not (
            gate.start_byte
            <= event.span.start_byte
            < event.span.end_byte
            <= gate.end_byte
        )
        for event in events
    ):
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}: permission gate does not cover every tab entry"
        )
    return True


def _validate_search_body(
    source: JavaSource,
    body: Node,
    output_name: str,
) -> None:
    expected = _compact(
        "{"
        "Set<ItemStack> tempItems = ItemStackLinkedSet.createTypeAndComponentsSet();"
        "for (CreativeModeTab tab : registry) {"
        "if (tab.getType() != CreativeModeTab.Type.SEARCH) {"
        "tempItems.addAll(tab.getSearchTabDisplayItems());"
        "}"
        "}"
        f"{output_name}.acceptAll(tempItems);"
        "}"
    )
    if _compact(source.text(body)) != expected:
        raise CreativeModeTabCatalogExtractionError(
            f"{source.relative_path}:{body.start_point.row + 1}:"
            f"{body.start_point.column + 1}: search tab union body drifted"
        )


def _display_entries(
    source: JavaSource,
    display_call: JavaCall,
    tab_type: str,
    item_resources: dict[str, tuple[str, ...]],
) -> tuple[bool, tuple[CreativeModeTabEntry, ...]]:
    if len(display_call.arguments) != 1:
        _fail(source, source.expression(display_call.node), "displayItems arity drifted")
    parameters_name, output_name, body = _lambda_parts(
        source, display_call.arguments[0]
    )
    calls = _calls_within(source, body.start_byte, body.end_byte)
    output_accepts = [
        call
        for call in calls
        if call.name == "accept"
        and call.qualifier is not None
        and call.qualifier.canonical == output_name
    ]
    output_accept_all = [
        call
        for call in calls
        if call.name == "acceptAll"
        and call.qualifier is not None
        and call.qualifier.canonical == output_name
    ]
    helpers = [
        call
        for call in calls
        if call.name in DYNAMIC_HELPERS
        and call.arguments
        and call.arguments[0].canonical == output_name
    ]
    unknown_output_consumers = [
        call
        for call in calls
        if call.arguments
        and call.arguments[0].canonical == output_name
        and call.name not in DYNAMIC_HELPERS
    ]
    if unknown_output_consumers:
        _fail(
            source,
            source.expression(unknown_output_consumers[0].node),
            "unknown creative output helper",
        )

    if tab_type == "search":
        if output_accepts or helpers or len(output_accept_all) != 1:
            _fail(
                source,
                display_call.arguments[0],
                "search tab is not the one acceptAll union",
            )
        accepted = output_accept_all[0]
        if len(accepted.arguments) != 1 or accepted.arguments[0].canonical != "tempItems":
            _fail(source, source.expression(accepted.node), "search union source drifted")
        _validate_search_body(source, body, output_name)
        return False, ()
    if output_accept_all:
        _fail(source, source.expression(output_accept_all[0].node), "unexpected acceptAll")

    events = sorted(
        output_accepts + helpers,
        key=lambda call: (call.span.start_byte, call.span.end_byte),
    )
    entries = tuple(
        _accept_entry(source, event, item_resources)
        if event.name == "accept"
        else _helper_entry(source, event, output_name, item_resources)
        for event in events
    )
    seen_items: set[str] = set()
    for entry in entries:
        if (
            entry.kind == "item"
            and entry.visibility != "search_only"
            and entry.item_resource_id in seen_items
        ):
            _fail(
                source,
                display_call.arguments[0],
                f"duplicate parent item {entry.item_resource_id}",
            )
        if entry.kind == "item" and entry.visibility != "search_only":
            if entry.item_resource_id is None:
                raise CreativeModeTabCatalogExtractionError(
                    f"{source.relative_path}: static creative item lacks a resource id"
                )
            seen_items.add(entry.item_resource_id)
    return (
        _event_permission_gate(source, body, events, parameters_name),
        entries,
    )


def extract_creative_mode_tab_catalog(
    tabs_source: JavaSource,
    tab_source: JavaSource,
    items_source: JavaSource,
    item_ids_source: JavaSource,
    expected_tab_count: int = 14,
    expected_static_entry_count: int = 1674,
    expected_dynamic_entry_count: int = 16,
) -> tuple[CreativeModeTabDefinition, ...]:
    default_background = _validate_creative_tab_defaults(tab_source)
    keys = _tab_keys(tabs_source)
    textures = _texture_fields(tabs_source)
    item_resources = _item_resources(items_source, item_ids_source)

    bootstrap = [
        method
        for method in tabs_source.methods_for(CREATIVE_TABS_OWNER)
        if method.name == "bootstrap" and method.body is not None
    ]
    if len(bootstrap) != 1:
        raise CreativeModeTabCatalogExtractionError(
            f"{tabs_source.relative_path}: expected one CreativeModeTabs.bootstrap"
        )
    body = bootstrap[0].body
    if body is None:
        raise CreativeModeTabCatalogExtractionError(
            f"{tabs_source.relative_path}: CreativeModeTabs.bootstrap lacks a body"
        )
    registrations = [
        call
        for call in _calls_within(tabs_source, body.start_byte, body.end_byte)
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "Registry"
    ]
    if len(registrations) != expected_tab_count:
        raise CreativeModeTabCatalogExtractionError(
            f"{tabs_source.relative_path}: expected {expected_tab_count} tab "
            f"registrations, extracted {len(registrations)}"
        )

    tabs: list[CreativeModeTabDefinition] = []
    registered_keys: set[str] = set()
    for registration in registrations:
        if len(registration.arguments) != 3 or registration.arguments[0].canonical != "registry":
            _fail(
                tabs_source,
                tabs_source.expression(registration.node),
                "unsupported creative tab registration",
            )
        key_name = registration.arguments[1].canonical
        resource_id = keys.get(key_name)
        if resource_id is None:
            _fail(
                tabs_source,
                registration.arguments[1],
                f"unresolved creative tab key {key_name}",
            )
        if resource_id in registered_keys:
            _fail(
                tabs_source,
                registration.arguments[1],
                f"duplicate creative tab registration {resource_id}",
            )
        registered_keys.add(resource_id)
        metadata, display_call = _builder_metadata(
            tabs_source,
            registration.arguments[2],
            item_resources,
            textures,
            default_background,
        )
        if metadata["tab_type"] in {"hotbar", "inventory"}:
            if display_call is not None:
                _fail(
                    tabs_source,
                    registration.arguments[2],
                    "hotbar/inventory tab unexpectedly has display items",
                )
            requires_permissions, entries = False, ()
        else:
            if display_call is None:
                _fail(
                    tabs_source,
                    registration.arguments[2],
                    "category/search tab lacks display items",
                )
            requires_permissions, entries = _display_entries(
                tabs_source,
                display_call,
                metadata["tab_type"],
                item_resources,
            )
        tabs.append(
            CreativeModeTabDefinition(
                resource_id=resource_id,
                requires_permissions=requires_permissions,
                entries=entries,
                **metadata,
            )
        )

    if set(keys.values()) != registered_keys:
        missing = sorted(set(keys.values()) - registered_keys)
        raise CreativeModeTabCatalogExtractionError(
            f"{tabs_source.relative_path}: unregistered creative tab keys {missing}"
        )
    static_count = sum(
        entry.kind == "item" for tab in tabs for entry in tab.entries
    )
    dynamic_count = sum(
        entry.kind != "item" for tab in tabs for entry in tab.entries
    )
    if static_count != expected_static_entry_count:
        raise CreativeModeTabCatalogExtractionError(
            f"{tabs_source.relative_path}: expected {expected_static_entry_count} "
            f"static entries, extracted {static_count}"
        )
    if dynamic_count != expected_dynamic_entry_count:
        raise CreativeModeTabCatalogExtractionError(
            f"{tabs_source.relative_path}: expected {expected_dynamic_entry_count} "
            f"dynamic entries, extracted {dynamic_count}"
        )
    return tuple(tabs)
