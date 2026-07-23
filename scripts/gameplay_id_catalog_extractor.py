"""Tree-sitter-backed domain extraction for vanilla gameplay numeric ids."""

from __future__ import annotations

import ast
import json
import math
import struct
from dataclasses import dataclass

from vanilla_java_source import (
    JavaExpression,
    JavaField,
    JavaSource,
    JavaType,
    descendants,
    type_body_members,
)


class GameplayIdExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class AttributeEntry:
    constant_name: str
    resource_id: str
    description_key: str
    default_value_bits: int
    sentiment: str
    syncable: bool


@dataclass(frozen=True)
class MobEffectEntry:
    constant_name: str
    resource_id: str
    category: str
    attribute_modifiers: tuple["MobEffectAttributeModifierEntry", ...]


@dataclass(frozen=True)
class MobEffectAttributeModifierEntry:
    attribute_constant_name: str
    modifier_id: str
    amount_bits: int
    operation_constant_name: str


@dataclass(frozen=True)
class BeaconEffectReference:
    constant_name: str
    expression: JavaExpression


@dataclass(frozen=True)
class MenuTypeEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class EquipmentSlotEntry:
    constant_name: str
    slot_type: str
    index: int
    count_limit: int
    slot_id: int
    serialized_name: str


@dataclass(frozen=True)
class EquipmentSlotGroupEntry:
    constant_name: str
    group_id: int
    serialized_name: str


@dataclass(frozen=True)
class EntityPoseEntry:
    constant_name: str
    pose_id: int
    serialized_name: str


@dataclass(frozen=True)
class EntityDataSerializerEntry:
    constant_name: str


@dataclass(frozen=True)
class ClientGameEventTypeEntry:
    constant_name: str
    event_id: int


@dataclass(frozen=True)
class SoundSourceEntry:
    constant_name: str
    serialized_name: str


@dataclass(frozen=True)
class HeightmapTypeEntry:
    constant_name: str
    type_id: int
    serialization_key: str
    usage: str


@dataclass(frozen=True)
class HeightmapDebugColorEntry:
    constant_name: str
    color_rgba: tuple[int, int, int, int]


@dataclass(frozen=True)
class GameTypeEntry:
    constant_name: str
    game_type_id: int
    serialized_name: str


@dataclass(frozen=True)
class PlayerInfoActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class DifficultyEntry:
    constant_name: str
    difficulty_id: int
    serialized_name: str


@dataclass(frozen=True)
class ResourcePackResponseActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class CustomChatCompletionsActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class ClientCommandActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class PlayerActionKindEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class PlayerCommandActionEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class EntityAnchorEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class RemoteDebugSampleTypeEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class DebugSubscriptionEntry:
    constant_name: str
    subscription_id: int
    resource_id: str
    has_value_codec: bool
    expire_after_ticks: int


@dataclass(frozen=True)
class DebugHolderTypeEntry:
    constant_name: str
    holder_id: int
    resource_id: str


@dataclass(frozen=True)
class WorldClockEntry:
    constant_name: str
    clock_id: int
    resource_id: str


@dataclass(frozen=True)
class GameRuleEntry:
    constant_name: str
    resource_id: str
    category: str
    value_kind: str
    min_value: int | None
    max_value: int | None


@dataclass(frozen=True)
class StatTypeEntry:
    constant_name: str
    type_id: int
    resource_id: str
    value_registry: str


@dataclass(frozen=True)
class CustomStatEntry:
    constant_name: str
    value_id: int
    resource_id: str
    formatter: str


@dataclass(frozen=True)
class ObjectiveRenderTypeEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class AdvancementFrameTypeEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class BossBarStyleEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class ScoreboardDisplaySlotEntry:
    constant_name: str
    slot_id: int
    serialized_name: str


@dataclass(frozen=True)
class ScoreboardTeamOptionEntry:
    constant_name: str
    option_id: int
    serialized_name: str


@dataclass(frozen=True)
class ChatFormattingEntry:
    constant_name: str
    ordinal: int
    serialized_name: str
    legacy_code: str
    formatting_id: int
    is_format: bool
    color: int | None


@dataclass(frozen=True)
class ScoreboardMethodEntry:
    constant_name: str
    method_id: int


@dataclass(frozen=True)
class BossEventOperationTypeEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class FilterMaskTypeEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class WaypointOperationEntry:
    constant_name: str
    ordinal: int
    serialized_name: str


@dataclass(frozen=True)
class WaypointDataTypeEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class InteractionHandEntry:
    constant_name: str
    hand_id: int


@dataclass(frozen=True)
class ClientInformationOptionEntry:
    constant_name: str
    option_id: int


@dataclass(frozen=True)
class ContainerInputEntry:
    constant_name: str
    input_id: int


@dataclass(frozen=True)
class RecipeBookTypeEntry:
    constant_name: str
    ordinal: int


@dataclass(frozen=True)
class RecipeBookCategoryEntry:
    constant_name: str
    category_id: int
    resource_id: str


@dataclass(frozen=True)
class RecipeBookSearchCategoryEntry:
    constant_name: str
    included_category_constants: tuple[str, ...]


@dataclass(frozen=True)
class RecipeDisplayTypeEntry:
    constant_name: str
    type_id: int
    resource_id: str


@dataclass(frozen=True)
class SlotDisplayTypeEntry:
    constant_name: str
    type_id: int
    resource_id: str


@dataclass(frozen=True)
class NamedComponentValueEntry:
    constant_name: str
    value_id: int
    serialized_name: str


@dataclass(frozen=True)
class ComponentValueEntry:
    constant_name: str
    value_id: int


@dataclass(frozen=True)
class ConsumeEffectTypeEntry:
    constant_name: str
    type_id: int
    resource_id: str


@dataclass(frozen=True)
class BuiltinEnumValueTypeEntry:
    constant_name: str
    strategy: str
    default_id: int
    values: tuple[NamedComponentValueEntry, ...]
    texture_diffuse_colors: tuple[int, ...] | None = None


@dataclass(frozen=True)
class BuiltinEnumComponentEntry:
    constant_name: str
    value_type_constant_name: str


@dataclass(frozen=True)
class TropicalFishPatternEntry:
    constant_name: str
    serialized_name: str
    base_id: int
    pattern_index: int
    packed_id: int


@dataclass(frozen=True)
class TropicalFishCommonVariantEntry:
    pattern_packed_id: int
    base_color_id: int
    pattern_color_id: int


@dataclass(frozen=True)
class MapColorEntry:
    constant_name: str
    color_id: int
    rgb: int


@dataclass(frozen=True)
class MapBrightnessEntry:
    constant_name: str
    brightness_id: int
    modifier: int


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise GameplayIdExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected string literal")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str):
        _fail(source, expression, "expected string literal")
    return value


def _integer(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type not in {"decimal_integer_literal", "hex_integer_literal"}:
        _fail(source, expression, "expected integer literal")
    try:
        return int(expression.canonical.replace("_", ""), 0)
    except ValueError:
        _fail(source, expression, "invalid integer literal")


def _double_literal_bits(source: JavaSource, expression: JavaExpression) -> int:
    literal = expression.canonical.replace("_", "").replace(" ", "")
    suffix = literal[-1:] if literal[-1:] in {"f", "F", "d", "D"} else ""
    text = literal[:-1] if suffix else literal
    try:
        value = float(text)
    except ValueError:
        _fail(source, expression, "expected numeric literal")
    if suffix in {"f", "F"}:
        value = struct.unpack("!f", struct.pack("!f", value))[0]
    if not math.isfinite(value):
        _fail(source, expression, "numeric literal must be finite")
    return struct.unpack("!Q", struct.pack("!d", value))[0]


def _signed_integer(source: JavaSource, expression: JavaExpression) -> int:
    canonical = expression.canonical.replace("_", "").replace(" ", "")
    try:
        return int(canonical, 0)
    except ValueError:
        _fail(source, expression, "expected signed integer literal")


def _character(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "character_literal":
        _fail(source, expression, "expected character literal")
    try:
        value = ast.literal_eval(source.text(expression.node))
    except (SyntaxError, ValueError) as error:
        _fail(source, expression, f"invalid Java character literal: {error}")
    if not isinstance(value, str) or len(value) != 1:
        _fail(source, expression, "expected one-character literal")
    return value


def _fields(source: JavaSource, prefix: str) -> list[JavaField]:
    return [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith(prefix)
    ]


def _register_arguments(source: JavaSource, field: JavaField) -> tuple[JavaExpression, ...]:
    initializer = field.initializer
    if initializer is None or initializer.node.type != "method_invocation":
        expression = initializer or JavaExpression("", field.span, field.node)
        _fail(source, expression, f"{field.name} lacks a register call")
    name = initializer.node.child_by_field_name("name")
    arguments = initializer.node.child_by_field_name("arguments")
    if name is None or arguments is None or source.text(name) != "register":
        _fail(source, initializer, f"{field.name} is not initialized by register")
    return tuple(source.expression(node) for node in arguments.named_children)


def _calls(source: JavaSource, expression: JavaExpression, name: str) -> list:
    return [
        call
        for call in source.calls
        if call.name == name
        and expression.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= expression.span.end_byte
    ]


def _object_arguments(source: JavaSource, expression: JavaExpression) -> tuple[JavaExpression, ...]:
    candidates = [
        node
        for node in [expression.node, *descendants(expression.node)]
        if node.type == "object_creation_expression"
    ]
    if len(candidates) != 1:
        _fail(source, expression, f"expected one object creation, found {len(candidates)}")
    arguments = candidates[0].child_by_field_name("arguments")
    if arguments is None:
        _fail(source, expression, "object creation lacks arguments")
    return tuple(source.expression(node) for node in arguments.named_children)


def _validate(entries: list, description: str, resource_attr: str | None = None) -> None:
    constants = [entry.constant_name for entry in entries]
    if len(constants) != len(set(constants)):
        raise GameplayIdExtractionError(f"duplicate {description} constant")
    if resource_attr is not None:
        values = [getattr(entry, resource_attr) for entry in entries]
        if len(values) != len(set(values)):
            raise GameplayIdExtractionError(f"duplicate {description} {resource_attr}")


def attribute_entries(source: JavaSource, expected_count: int = 35) -> list[AttributeEntry]:
    entries: list[AttributeEntry] = []
    for field in _fields(source, "Holder<Attribute>"):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, "attribute register must have two arguments")
        constructor = _object_arguments(source, arguments[1])
        if len(constructor) != 4:
            _fail(source, arguments[1], "ranged attribute constructor must have four arguments")
        sentiment_calls = _calls(source, field.initializer, "setSentiment")
        if len(sentiment_calls) > 1 or (sentiment_calls and len(sentiment_calls[0].arguments) != 1):
            _fail(source, field.initializer, "ambiguous attribute sentiment")
        sentiment = "POSITIVE"
        if sentiment_calls:
            sentiment = sentiment_calls[0].arguments[0].canonical.rsplit(".", 1)[-1]
            if sentiment not in {"POSITIVE", "NEUTRAL", "NEGATIVE"}:
                _fail(source, sentiment_calls[0].arguments[0], "unknown attribute sentiment")
        sync_calls = _calls(source, field.initializer, "setSyncable")
        if len(sync_calls) > 1 or any(
            len(call.arguments) != 1 or call.arguments[0].canonical != "true"
            for call in sync_calls
        ):
            _fail(source, field.initializer, "unsupported attribute syncable expression")
        entries.append(
            AttributeEntry(
                field.name,
                "minecraft:" + _string(source, arguments[0]),
                _string(source, constructor[0]),
                _double_literal_bits(source, constructor[1]),
                sentiment,
                bool(sync_calls),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} attributes, extracted {len(entries)}"
        )
    _validate(entries, "attribute", "resource_id")
    return entries


def mob_effect_entries(
    source: JavaSource, expected_count: int = 40, expected_modifier_count: int = 12
) -> list[MobEffectEntry]:
    entries: list[MobEffectEntry] = []
    for field in _fields(source, "Holder<MobEffect>"):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, "mob effect register must have two arguments")
        categories = [
            source.expression(node).canonical.rsplit(".", 1)[-1]
            for node in descendants(arguments[1].node)
            if node.type in {"field_access", "scoped_identifier"}
            and source.expression(node).canonical.startswith("MobEffectCategory.")
        ]
        if len(categories) != 1 or categories[0] not in {"BENEFICIAL", "HARMFUL", "NEUTRAL"}:
            _fail(source, arguments[1], f"expected one mob effect category, found {categories}")
        modifiers = []
        for call in source.calls:
            if (
                call.name != "addAttributeModifier"
                or call.span.start_byte < arguments[1].span.start_byte
                or call.span.end_byte > arguments[1].span.end_byte
            ):
                continue
            if len(call.arguments) != 4:
                _fail(source, source.expression(call.node), "mob effect attribute modifier must have four arguments")
            attribute = call.arguments[0].canonical
            if not attribute.startswith("Attributes."):
                _fail(source, call.arguments[0], "mob effect modifier attribute must reference Attributes")
            modifier_id_call = call.arguments[1].node
            modifier_name = modifier_id_call.child_by_field_name("name")
            modifier_arguments = modifier_id_call.child_by_field_name("arguments")
            if (
                modifier_id_call.type != "method_invocation"
                or modifier_name is None
                or source.text(modifier_name) != "withDefaultNamespace"
                or modifier_arguments is None
                or len(modifier_arguments.named_children) != 1
            ):
                _fail(source, call.arguments[1], "mob effect modifier id must use Identifier.withDefaultNamespace")
            amount_literal = call.arguments[2].canonical.replace(" ", "")
            amount_text = amount_literal.removesuffix("F").removesuffix("D")
            try:
                amount = float(amount_text)
            except ValueError:
                _fail(source, call.arguments[2], "mob effect modifier amount must be a numeric literal")
            if amount_literal.endswith("F"):
                amount = struct.unpack("!f", struct.pack("!f", amount))[0]
            amount_bits = struct.unpack("!Q", struct.pack("!d", amount))[0]
            operation = call.arguments[3].canonical
            if not operation.startswith("AttributeModifier.Operation."):
                _fail(source, call.arguments[3], "mob effect modifier operation must reference AttributeModifier.Operation")
            modifiers.append(
                MobEffectAttributeModifierEntry(
                    attribute.rsplit(".", 1)[-1],
                    "minecraft:" + _string(
                        source, source.expression(modifier_arguments.named_children[0])
                    ),
                    amount_bits,
                    operation.rsplit(".", 1)[-1],
                )
            )
        entries.append(
            MobEffectEntry(
                field.name,
                "minecraft:" + _string(source, arguments[0]),
                categories[0],
                tuple(modifiers),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} mob effects, extracted {len(entries)}"
        )
    _validate(entries, "mob effect", "resource_id")
    if sum(len(entry.attribute_modifiers) for entry in entries) != expected_modifier_count:
        raise GameplayIdExtractionError(
            f"expected {expected_modifier_count} mob effect attribute modifiers"
        )
    return entries


def beacon_effect_tiers(
    source: JavaSource,
    expected_tier_count: int = 4,
    expected_effect_count: int = 6,
) -> list[tuple[BeaconEffectReference, ...]]:
    fields = [field for field in source.fields if field.name == "BEACON_EFFECTS"]
    if len(fields) != 1 or fields[0].initializer is None:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one initialized BEACON_EFFECTS field"
        )
    initializer = fields[0].initializer

    def list_arguments(expression: JavaExpression) -> tuple[JavaExpression, ...]:
        node = expression.node
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        object_node = node.child_by_field_name("object")
        if (
            node.type != "method_invocation"
            or name is None
            or source.text(name) != "of"
            or arguments is None
            or object_node is None
            or source.expression(object_node).canonical != "List"
        ):
            _fail(source, expression, "beacon effect tiers must use List.of")
        return tuple(source.expression(node) for node in arguments.named_children)

    tiers: list[tuple[BeaconEffectReference, ...]] = []
    for tier_expression in list_arguments(initializer):
        references = []
        for effect_expression in list_arguments(tier_expression):
            node = effect_expression.node
            object_node = node.child_by_field_name("object")
            field_node = node.child_by_field_name("field")
            if (
                node.type != "field_access"
                or object_node is None
                or field_node is None
                or source.expression(object_node).canonical != "MobEffects"
            ):
                _fail(
                    source,
                    effect_expression,
                    "beacon effect must reference a MobEffects constant",
                )
            references.append(
                BeaconEffectReference(source.text(field_node), effect_expression)
            )
        tiers.append(tuple(references))

    if len(tiers) != expected_tier_count:
        _fail(
            source,
            initializer,
            f"expected {expected_tier_count} beacon effect tiers, extracted {len(tiers)}",
        )
    effect_count = sum(len(tier) for tier in tiers)
    if effect_count != expected_effect_count:
        _fail(
            source,
            initializer,
            f"expected {expected_effect_count} beacon effects, extracted {effect_count}",
        )
    return tiers


def menu_type_entries(source: JavaSource, expected_count: int = 25) -> list[MenuTypeEntry]:
    entries = []
    for field in _fields(source, "MenuType<"):
        arguments = _register_arguments(source, field)
        if not arguments:
            _fail(source, field.initializer, "menu register lacks arguments")
        entries.append(MenuTypeEntry(field.name, "minecraft:" + _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} menu types, extracted {len(entries)}"
        )
    _validate(entries, "menu type", "resource_id")
    return entries


def _enum_constants(source: JavaSource) -> list[tuple[str, tuple[JavaExpression, ...]]]:
    enums = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.node.parent.type == "program"
    ]
    if len(enums) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one enum, found {len(enums)}"
        )
    result = []
    for node in type_body_members(enums[0].body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None:
            _fail(source, source.expression(node), "enum constant is incomplete")
        result.append(
            (
                source.text(name),
                tuple(
                    source.expression(child)
                    for child in (() if arguments is None else arguments.named_children)
                ),
            )
        )
    return result


def map_color_entries(
    source: JavaSource,
    expected_color_count: int = 62,
    expected_brightness_count: int = 4,
    expected_palette_size: int = 64,
) -> tuple[list[MapColorEntry], list[MapBrightnessEntry], int]:
    owners = [
        java_type
        for java_type in source.types
        if java_type.kind == "class" and java_type.simple_name == "MapColor"
    ]
    if len(owners) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one MapColor class, found {len(owners)}"
        )
    palette_fields = [
        field for field in source.fields_for(owners[0]) if field.name == "MATERIAL_COLORS"
    ]
    if len(palette_fields) != 1 or palette_fields[0].initializer is None:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one initialized MATERIAL_COLORS field"
        )
    palette_size_nodes = [
        node
        for node in descendants(palette_fields[0].initializer.node)
        if node.type in {"decimal_integer_literal", "hex_integer_literal"}
    ]
    if len(palette_size_nodes) != 1:
        _fail(
            source,
            palette_fields[0].initializer,
            "MATERIAL_COLORS must have one literal array size",
        )
    palette_size = _integer(source, source.expression(palette_size_nodes[0]))
    if palette_size != expected_palette_size:
        raise GameplayIdExtractionError(
            f"expected map color palette size {expected_palette_size}, extracted {palette_size}"
        )
    colors = []
    for field in source.fields_for(owners[0]):
        if field.type_spelling.replace(" ", "") != "MapColor":
            continue
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"map color {field.name} lacks an initializer",
            )
        arguments = _object_arguments(source, field.initializer)
        if len(arguments) != 2:
            _fail(
                source,
                field.initializer,
                f"map color {field.name} must have id and RGB arguments",
            )
        colors.append(
            MapColorEntry(
                field.name,
                _integer(source, arguments[0]),
                _integer(source, arguments[1]),
            )
        )
    if len(colors) != expected_color_count:
        raise GameplayIdExtractionError(
            f"expected {expected_color_count} map colors, extracted {len(colors)}"
        )
    _validate(colors, "map color", "color_id")
    if [entry.color_id for entry in colors] != list(range(expected_color_count)):
        raise GameplayIdExtractionError("map color ids do not follow declaration order")
    for entry in colors:
        if not 0 <= entry.rgb <= 0xFFFFFF:
            raise GameplayIdExtractionError(
                f"map color {entry.constant_name} RGB is outside 24-bit range"
            )

    brightness_types = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == "Brightness"
    ]
    if len(brightness_types) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one Brightness enum, found {len(brightness_types)}"
        )
    brightness = []
    for node in type_body_members(brightness_types[0].body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None:
            _fail(source, source.expression(node), "map brightness constant is incomplete")
        values = tuple(source.expression(child) for child in arguments.named_children)
        if len(values) != 2:
            _fail(
                source,
                source.expression(node),
                "map brightness must have id and modifier arguments",
            )
        brightness.append(
            MapBrightnessEntry(
                source.text(name),
                _integer(source, values[0]),
                _integer(source, values[1]),
            )
        )
    if len(brightness) != expected_brightness_count:
        raise GameplayIdExtractionError(
            f"expected {expected_brightness_count} map brightness values, extracted {len(brightness)}"
        )
    _validate(brightness, "map brightness", "brightness_id")
    if [entry.brightness_id for entry in brightness] != list(
        range(expected_brightness_count)
    ):
        raise GameplayIdExtractionError("map brightness ids do not follow enum order")
    for entry in brightness:
        if not 0 <= entry.modifier <= 255:
            raise GameplayIdExtractionError(
                f"map brightness {entry.constant_name} modifier is outside byte range"
            )
    if colors[-1].color_id >= palette_size:
        raise GameplayIdExtractionError("map color id exceeds MATERIAL_COLORS capacity")
    return colors, brightness, palette_size


def heightmap_type_entries(
    source: JavaSource, expected_count: int = 6
) -> list[HeightmapTypeEntry]:
    enums = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == "Types"
    ]
    if len(enums) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one Heightmap.Types enum, found {len(enums)}"
        )
    entries = []
    for node in type_body_members(enums[0].body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None or len(arguments.named_children) != 4:
            _fail(source, source.expression(node), "heightmap type must have four arguments")
        values = tuple(source.expression(child) for child in arguments.named_children)
        usage = values[2].canonical.rsplit(".", 1)[-1]
        if usage not in {"WORLDGEN", "LIVE_WORLD", "CLIENT"}:
            _fail(source, values[2], f"unknown heightmap usage {usage}")
        entries.append(
            HeightmapTypeEntry(
                source.text(name),
                _integer(source, values[0]),
                _string(source, values[1]),
                usage,
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} heightmap types, extracted {len(entries)}"
        )
    _validate(entries, "heightmap type", "type_id")
    _validate(entries, "heightmap type", "serialization_key")
    if [entry.type_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("heightmap type ids are not contiguous declaration order")
    return entries


def _java_float_color_channel(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type != "decimal_floating_point_literal":
        _fail(source, expression, "heightmap debug color channel must be a float literal")
    literal = source.text(expression.node).replace("_", "")
    if not literal.endswith(("F", "f")):
        _fail(source, expression, "heightmap debug color channel must use float precision")
    try:
        value = float(literal[:-1])
    except ValueError:
        _fail(source, expression, "invalid heightmap debug color float literal")
    value = struct.unpack("!f", struct.pack("!f", value))[0]
    if not 0.0 <= value <= 1.0:
        _fail(source, expression, "heightmap debug color channel is outside 0..1")
    scaled = struct.unpack("!f", struct.pack("!f", value * 255.0))[0]
    return math.floor(scaled)


def heightmap_debug_color_entries(
    source: JavaSource, expected_count: int = 6
) -> list[HeightmapDebugColorEntry]:
    methods = [method for method in source.methods if method.name == "getColor"]
    if len(methods) != 1 or len(methods[0].parameters) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one single-parameter getColor method"
        )
    returns = [
        node for node in descendants(methods[0].node) if node.type == "return_statement"
    ]
    if (
        len(returns) != 1
        or len(returns[0].named_children) != 1
        or returns[0].named_children[0].type != "switch_expression"
    ):
        expression = source.expression(returns[0] if returns else methods[0].node)
        _fail(source, expression, "heightmap getColor must directly return one switch")
    switch = returns[0].named_children[0]
    if (
        len(switch.named_children) != 2
        or source.expression(switch.named_children[0]).canonical != "(type)"
        or switch.named_children[1].type != "switch_block"
    ):
        _fail(source, source.expression(switch), "heightmap color switch shape changed")

    entries = []
    for rule in switch.named_children[1].named_children:
        if rule.type != "switch_rule":
            _fail(
                source,
                source.expression(rule),
                "heightmap color switch must contain only rules",
            )
        labels = [child for child in rule.named_children if child.type == "switch_label"]
        creations = [
            node for node in descendants(rule) if node.type == "object_creation_expression"
        ]
        if len(labels) != 1 or len(labels[0].named_children) != 1:
            _fail(
                source,
                source.expression(rule),
                "heightmap color rule must have one case label",
            )
        label = labels[0].named_children[0]
        if label.type != "identifier" or len(creations) != 1:
            _fail(
                source,
                source.expression(rule),
                "heightmap color rule must create one Vector3f",
            )
        creation = creations[0]
        type_node = creation.child_by_field_name("type")
        arguments = creation.child_by_field_name("arguments")
        if (
            type_node is None
            or source.expression(type_node).canonical != "Vector3f"
            or arguments is None
            or len(arguments.named_children) != 3
        ):
            _fail(
                source,
                source.expression(creation),
                "heightmap color must be a three-channel Vector3f",
            )
        channels = tuple(
            _java_float_color_channel(source, source.expression(argument))
            for argument in arguments.named_children
        )
        entries.append(
            HeightmapDebugColorEntry(source.text(label), (*channels, 255))
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} heightmap debug colors, extracted {len(entries)}"
        )
    _validate(entries, "heightmap debug color")
    return entries


def game_type_entries(
    source: JavaSource, expected_count: int = 4
) -> list[GameTypeEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            _fail(source, arguments[0], f"game type {name} must have two arguments")
        entries.append(
            GameTypeEntry(
                name,
                _integer(source, arguments[0]),
                _string(source, arguments[1]),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} game types, extracted {len(entries)}"
        )
    _validate(entries, "game type", "game_type_id")
    _validate(entries, "game type", "serialized_name")
    if [entry.game_type_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("game type ids are not contiguous declaration order")
    return entries


def game_type_not_set_id(source: JavaSource) -> int:
    fields = [field for field in source.fields if field.name == "NOT_SET"]
    if len(fields) != 1 or fields[0].initializer is None:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one initialized NOT_SET field"
        )
    initializer = fields[0].initializer
    if initializer.canonical.replace(" ", "") != "-1":
        _fail(source, initializer, "game type NOT_SET must be -1")
    return int(initializer.canonical.replace(" ", ""), 0)


def _nested_enum_type(source: JavaSource, enum_name: str) -> JavaType:
    enums = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == enum_name
    ]
    if len(enums) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one nested {enum_name} enum, found {len(enums)}"
        )
    return enums[0]


def _nested_enum_constants(
    source: JavaSource, enum_name: str, description: str
) -> list[tuple[str, tuple[JavaExpression, ...]]]:
    enum_type = _nested_enum_type(source, enum_name)
    entries: list[tuple[str, tuple[JavaExpression, ...]]] = []
    for node in type_body_members(enum_type.body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        if name is None:
            _fail(source, source.expression(node), f"{description} lacks a name")
        arguments = node.child_by_field_name("arguments")
        entries.append(
            (
                source.text(name),
                tuple(
                    source.expression(child)
                    for child in (() if arguments is None else arguments.named_children)
                ),
            )
        )
    return entries


def _nested_enum_ordinals(
    source: JavaSource, enum_name: str, description: str, expected_count: int
) -> list[tuple[str, int]]:
    constants = _nested_enum_constants(source, enum_name, description)
    entries = [(name, ordinal) for ordinal, (name, _) in enumerate(constants)]
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} {description}s, extracted {len(entries)}"
        )
    return entries


def player_info_action_entries(
    source: JavaSource, expected_count: int = 8
) -> list[PlayerInfoActionEntry]:
    entries = [
        PlayerInfoActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "player-info action", expected_count
        )
    ]
    _validate(entries, "player-info action", "ordinal")
    return entries


def resource_pack_response_action_entries(
    source: JavaSource, expected_count: int = 8
) -> list[ResourcePackResponseActionEntry]:
    entries = [
        ResourcePackResponseActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "resource-pack response action", expected_count
        )
    ]
    _validate(entries, "resource-pack response action", "ordinal")
    return entries


def custom_chat_completions_action_entries(
    source: JavaSource, expected_count: int = 3
) -> list[CustomChatCompletionsActionEntry]:
    entries = [
        CustomChatCompletionsActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "custom-chat completions action", expected_count
        )
    ]
    _validate(entries, "custom-chat completions action", "ordinal")
    return entries


def client_command_action_entries(
    source: JavaSource, expected_count: int = 3
) -> list[ClientCommandActionEntry]:
    entries = [
        ClientCommandActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "client command action", expected_count
        )
    ]
    _validate(entries, "client command action", "ordinal")
    return entries


def player_action_kind_entries(
    source: JavaSource, expected_count: int = 8
) -> list[PlayerActionKindEntry]:
    entries = [
        PlayerActionKindEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "player action kind", expected_count
        )
    ]
    _validate(entries, "player action kind", "ordinal")
    return entries


def player_command_action_entries(
    source: JavaSource, expected_count: int = 7
) -> list[PlayerCommandActionEntry]:
    entries = [
        PlayerCommandActionEntry(name, ordinal)
        for name, ordinal in _nested_enum_ordinals(
            source, "Action", "player command action", expected_count
        )
    ]
    _validate(entries, "player command action", "ordinal")
    return entries


def entity_anchor_entries(
    source: JavaSource, expected_count: int = 2
) -> list[EntityAnchorEntry]:
    entries = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(source, "Anchor", "entity anchor")
    ):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"entity anchor {name} must have two arguments")
        entries.append(EntityAnchorEntry(name, ordinal, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} entity anchors, extracted {len(entries)}"
        )
    _validate(entries, "entity anchor", "ordinal")
    _validate(entries, "entity anchor", "serialized_name")
    return entries


def remote_debug_sample_type_entries(
    source: JavaSource, expected_count: int = 1
) -> list[RemoteDebugSampleTypeEntry]:
    entries = [
        RemoteDebugSampleTypeEntry(name, ordinal)
        for ordinal, (name, _) in enumerate(_enum_constants(source))
    ]
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} remote debug sample types, extracted {len(entries)}"
        )
    _validate(entries, "remote debug sample type", "ordinal")
    return entries


def debug_subscription_entries(
    source: JavaSource, expected_count: int = 16
) -> list[DebugSubscriptionEntry]:
    entries = []
    registration_methods = {
        "registerSimple",
        "registerTemporaryValue",
        "registerWithValue",
    }
    for field in _fields(source, "DebugSubscription<"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or JavaExpression("", field.span, field.node)
            _fail(source, expression, f"{field.name} lacks a debug subscription register call")
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if name is None or arguments is None:
            _fail(source, initializer, f"{field.name} has an incomplete register call")
        registration_method = source.text(name)
        if registration_method not in registration_methods:
            _fail(
                source,
                initializer,
                f"{field.name} uses unsupported debug subscription registration {registration_method}",
            )
        call_arguments = tuple(
            source.expression(node) for node in arguments.named_children
        )
        if not call_arguments:
            _fail(source, initializer, f"{field.name} register call lacks a resource id")
        expected_arguments = {
            "registerSimple": 1,
            "registerWithValue": 2,
            "registerTemporaryValue": 3,
        }[registration_method]
        if len(call_arguments) != expected_arguments:
            _fail(
                source,
                initializer,
                f"{field.name} {registration_method} must have {expected_arguments} arguments",
            )
        expire_after_ticks = (
            _integer(source, call_arguments[2])
            if registration_method == "registerTemporaryValue"
            else 0
        )
        if expire_after_ticks < 0:
            _fail(source, call_arguments[2], "debug subscription expiry must be non-negative")
        entries.append(
            DebugSubscriptionEntry(
                field.name,
                len(entries),
                "minecraft:" + _string(source, call_arguments[0]),
                registration_method != "registerSimple",
                expire_after_ticks,
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} debug subscriptions, extracted {len(entries)}"
        )
    _validate(entries, "debug subscription", "subscription_id")
    _validate(entries, "debug subscription", "resource_id")
    return entries


def debug_game_event_entries(
    source: JavaSource, expected_count: int = 60
) -> list[DebugHolderTypeEntry]:
    entries = []
    for field in _fields(source, "Holder.Reference<GameEvent>"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or JavaExpression("", field.span, field.node)
            _fail(source, expression, f"{field.name} lacks a game event register call")
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if name is None or arguments is None or source.text(name) != "register":
            _fail(source, initializer, f"{field.name} is not initialized by register")
        call_arguments = tuple(
            source.expression(node) for node in arguments.named_children
        )
        if len(call_arguments) not in {1, 2}:
            _fail(source, initializer, f"{field.name} register must have one or two arguments")
        entries.append(
            DebugHolderTypeEntry(
                field.name,
                len(entries),
                "minecraft:" + _string(source, call_arguments[0]),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} debug game events, extracted {len(entries)}"
        )
    _validate(entries, "debug game event", "holder_id")
    _validate(entries, "debug game event", "resource_id")
    return entries


def debug_poi_type_entries(
    source: JavaSource, expected_count: int = 21
) -> list[DebugHolderTypeEntry]:
    keys: dict[str, str] = {}
    for field in _fields(source, "ResourceKey<PoiType>"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or JavaExpression("", field.span, field.node)
            _fail(source, expression, f"{field.name} lacks a POI key call")
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if name is None or arguments is None or source.text(name) != "createKey":
            _fail(source, initializer, f"{field.name} is not initialized by createKey")
        call_arguments = tuple(
            source.expression(node) for node in arguments.named_children
        )
        if len(call_arguments) != 1:
            _fail(source, initializer, f"{field.name} key call must have one argument")
        keys[field.name] = "minecraft:" + _string(source, call_arguments[0])

    entries = []
    used_keys = set()
    for call in source.calls:
        if call.name != "register" or call.qualifier is not None or len(call.arguments) != 5:
            continue
        constant_name = call.arguments[1].canonical
        if constant_name not in keys:
            _fail(source, call.arguments[1], f"POI register references unknown key {constant_name}")
        if constant_name in used_keys:
            _fail(source, call.arguments[1], f"duplicate POI register {constant_name}")
        used_keys.add(constant_name)
        entries.append(
            DebugHolderTypeEntry(constant_name, len(entries), keys[constant_name])
        )

    unused_keys = sorted(set(keys) - used_keys)
    if unused_keys:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: unregistered POI keys: {', '.join(unused_keys)}"
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} debug POI types, extracted {len(entries)}"
        )
    _validate(entries, "debug POI type", "holder_id")
    _validate(entries, "debug POI type", "resource_id")
    return entries


def world_clock_entries(
    source: JavaSource, expected_count: int = 2
) -> list[WorldClockEntry]:
    keys: dict[str, str] = {}
    for field in _fields(source, "ResourceKey<WorldClock>"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or JavaExpression("", field.span, field.node)
            _fail(source, expression, f"{field.name} lacks a world clock key call")
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if name is None or arguments is None or source.text(name) != "key":
            _fail(source, initializer, f"{field.name} is not initialized by key")
        call_arguments = tuple(
            source.expression(node) for node in arguments.named_children
        )
        if len(call_arguments) != 1:
            _fail(source, initializer, f"{field.name} key call must have one argument")
        keys[field.name] = "minecraft:" + _string(source, call_arguments[0])

    entries = []
    used_keys = set()
    for call in source.calls:
        if (
            call.name != "register"
            or call.qualifier is None
            or call.qualifier.canonical != "context"
        ):
            continue
        if len(call.arguments) != 2:
            _fail(source, source.expression(call.node), "world clock register must have two arguments")
        constant_name = call.arguments[0].canonical
        if constant_name not in keys:
            _fail(
                source,
                call.arguments[0],
                f"world clock register references unknown key {constant_name}",
            )
        if constant_name in used_keys:
            _fail(source, call.arguments[0], f"duplicate world clock register {constant_name}")
        used_keys.add(constant_name)
        entries.append(
            WorldClockEntry(constant_name, len(entries), keys[constant_name])
        )

    unused_keys = sorted(set(keys) - used_keys)
    if unused_keys:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: unregistered world clock keys: {', '.join(unused_keys)}"
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} world clocks, extracted {len(entries)}"
        )
    _validate(entries, "world clock", "clock_id")
    _validate(entries, "world clock", "resource_id")
    return entries


def game_rule_entries(
    source: JavaSource, expected_count: int = 59
) -> list[GameRuleEntry]:
    entries = []
    for field in _fields(source, "GameRule<"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or JavaExpression("", field.span, field.node)
            _fail(source, expression, f"{field.name} lacks a game-rule register call")
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if name is None or arguments is None:
            _fail(source, initializer, f"{field.name} has an incomplete register call")
        method = source.text(name)
        call_arguments = tuple(
            source.expression(node) for node in arguments.named_children
        )
        value_type = field.type_spelling.replace(" ", "")
        if method == "registerBoolean":
            if value_type != "GameRule<Boolean>" or len(call_arguments) != 3:
                _fail(source, initializer, f"{field.name} boolean registration changed")
            value_kind = "boolean"
            min_value = None
            max_value = None
        elif method == "registerInteger":
            if value_type != "GameRule<Integer>" or len(call_arguments) not in {4, 5, 6}:
                _fail(source, initializer, f"{field.name} integer registration changed")
            value_kind = "integer"
            min_value = _signed_integer(source, call_arguments[3])
            max_value = (
                _signed_integer(source, call_arguments[4])
                if len(call_arguments) >= 5
                else 2_147_483_647
            )
            if min_value > max_value:
                _fail(source, initializer, f"{field.name} has an inverted integer range")
        else:
            _fail(source, initializer, f"{field.name} uses unsupported {method}")
        category = call_arguments[1].canonical
        if not category.startswith("GameRuleCategory."):
            _fail(
                source,
                call_arguments[1],
                f"{field.name} category must reference GameRuleCategory",
            )
        category = category.removeprefix("GameRuleCategory.").lower()
        entries.append(
            GameRuleEntry(
                field.name,
                "minecraft:" + _string(source, call_arguments[0]),
                category,
                value_kind,
                min_value,
                max_value,
            )
        )

    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} game rules, extracted {len(entries)}"
        )
    _validate(entries, "game rule", "resource_id")
    return entries


def stat_catalog_entries(
    source: JavaSource,
    expected_type_count: int = 9,
    expected_custom_count: int = 77,
) -> tuple[list[StatTypeEntry], list[CustomStatEntry]]:
    registry_names = {
        "BuiltInRegistries.BLOCK": "minecraft:block",
        "BuiltInRegistries.ITEM": "minecraft:item",
        "BuiltInRegistries.ENTITY_TYPE": "minecraft:entity_type",
        "BuiltInRegistries.CUSTOM_STAT": "minecraft:custom_stat",
    }
    types = []
    for field in _fields(source, "StatType<"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            expression = initializer or JavaExpression("", field.span, field.node)
            _fail(source, expression, f"{field.name} lacks makeRegistryStatType")
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if name is None or arguments is None or source.text(name) != "makeRegistryStatType":
            _fail(source, initializer, f"{field.name} is not initialized by makeRegistryStatType")
        call_arguments = tuple(source.expression(node) for node in arguments.named_children)
        if len(call_arguments) != 2:
            _fail(source, initializer, f"{field.name} stat type call must have two arguments")
        registry = registry_names.get(call_arguments[1].canonical)
        if registry is None:
            _fail(source, call_arguments[1], f"{field.name} has unsupported value registry")
        types.append(StatTypeEntry(
            field.name, len(types), "minecraft:" + _string(source, call_arguments[0]), registry
        ))
    custom = []
    for field in _fields(source, "Identifier"):
        initializer = field.initializer
        if initializer is None or initializer.node.type != "method_invocation":
            continue
        name = initializer.node.child_by_field_name("name")
        arguments = initializer.node.child_by_field_name("arguments")
        if name is None or arguments is None or source.text(name) != "makeCustomStat":
            continue
        call_arguments = tuple(source.expression(node) for node in arguments.named_children)
        if len(call_arguments) != 2:
            _fail(source, initializer, f"{field.name} custom stat call must have two arguments")
        formatter = call_arguments[1].canonical.removeprefix("StatFormatter.")
        if formatter not in {"DEFAULT", "TIME", "DISTANCE", "DIVIDE_BY_TEN"}:
            _fail(source, call_arguments[1], f"{field.name} has unsupported formatter")
        custom.append(CustomStatEntry(
            field.name, len(custom), "minecraft:" + _string(source, call_arguments[0]), formatter
        ))
    if len(types) != expected_type_count or len(custom) != expected_custom_count:
        raise GameplayIdExtractionError(
            f"expected {expected_type_count} stat types/{expected_custom_count} custom stats, "
            f"extracted {len(types)}/{len(custom)}"
        )
    _validate(types, "stat type", "type_id")
    _validate(types, "stat type", "resource_id")
    _validate(custom, "custom stat", "value_id")
    _validate(custom, "custom stat", "resource_id")
    return types, custom


def objective_render_type_entries(
    source: JavaSource, expected_count: int = 2
) -> list[ObjectiveRenderTypeEntry]:
    entries = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(source, "RenderType", "objective render type")
    ):
        if len(arguments) != 1:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"objective render type {name} must have one argument")
        entries.append(ObjectiveRenderTypeEntry(name, ordinal, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} objective render types, extracted {len(entries)}"
        )
    _validate(entries, "objective render type", "ordinal")
    _validate(entries, "objective render type", "serialized_name")
    return entries


def objective_criteria_names(
    source: JavaSource,
    chat_formatting: list[ChatFormattingEntry],
    expected_count: int = 43,
) -> list[str]:
    colors = {
        entry.constant_name: entry.serialized_name
        for entry in chat_formatting
        if not entry.is_format
    }
    names = []
    for field in _fields(source, "ObjectiveCriteria"):
        initializer = field.initializer
        if initializer is None:
            continue
        for call in _calls(source, initializer, "registerCustom"):
            if not call.arguments:
                _fail(source, initializer, f"{field.name} registerCustom lacks a name")
            name = _objective_criteria_name(source, call.arguments[0], colors)
            names.append(name)
    if len(names) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} objective criteria, extracted {len(names)}"
        )
    if len(set(names)) != len(names):
        raise GameplayIdExtractionError("objective criteria names are not unique")
    return names


def _objective_criteria_name(
    source: JavaSource,
    expression: JavaExpression,
    colors: dict[str, str],
) -> str:
    if expression.node.type == "string_literal":
        return _string(source, expression)
    if expression.node.type != "binary_expression":
        _fail(source, expression, "unsupported objective criteria name")
    left = expression.node.child_by_field_name("left")
    right = expression.node.child_by_field_name("right")
    if left is None or right is None:
        _fail(source, expression, "incomplete objective criteria name")
    prefix = _string(source, source.expression(left))
    if right.type != "method_invocation":
        _fail(source, source.expression(right), "objective criteria suffix is not getName")
    name = right.child_by_field_name("name")
    object_node = right.child_by_field_name("object")
    if name is None or source.text(name) != "getName" or object_node is None:
        _fail(source, source.expression(right), "objective criteria suffix is not getName")
    color_ref = source.expression(object_node).canonical
    color_name = color_ref.removeprefix("ChatFormatting.")
    if color_name not in colors or color_ref == color_name:
        _fail(source, source.expression(object_node), "unknown objective criteria color")
    return prefix + colors[color_name]


def advancement_frame_type_entries(
    source: JavaSource, expected_count: int = 3
) -> list[AdvancementFrameTypeEntry]:
    entries = []
    for ordinal, (name, arguments) in enumerate(_enum_constants(source)):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"advancement frame type {name} must have two arguments")
        entries.append(AdvancementFrameTypeEntry(name, ordinal, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} advancement frame types, extracted {len(entries)}"
        )
    _validate(entries, "advancement frame type", "ordinal")
    _validate(entries, "advancement frame type", "serialized_name")
    return entries


def boss_bar_style_entries(
    source: JavaSource,
) -> tuple[list[BossBarStyleEntry], list[BossBarStyleEntry]]:
    def extract(enum_name: str, expected_count: int) -> list[BossBarStyleEntry]:
        entries = []
        for ordinal, (name, arguments) in enumerate(
            _nested_enum_constants(source, enum_name, enum_name)
        ):
            if not arguments:
                _fail(source, source.expression(source.types[0].node), f"{enum_name} {name} lacks name")
            entries.append(BossBarStyleEntry(name, ordinal, _string(source, arguments[0])))
        if len(entries) != expected_count:
            raise GameplayIdExtractionError(
                f"expected {expected_count} {enum_name} values, extracted {len(entries)}"
            )
        _validate(entries, enum_name, "ordinal")
        _validate(entries, enum_name, "serialized_name")
        return entries

    return extract("BossBarColor", 7), extract("BossBarOverlay", 5)


def difficulty_entries(
    source: JavaSource, expected_count: int = 4
) -> list[DifficultyEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            _fail(source, arguments[0], f"difficulty {name} must have two arguments")
        entries.append(
            DifficultyEntry(
                name,
                _integer(source, arguments[0]),
                _string(source, arguments[1]),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} difficulties, extracted {len(entries)}"
        )
    _validate(entries, "difficulty", "difficulty_id")
    _validate(entries, "difficulty", "serialized_name")
    if [entry.difficulty_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("difficulty ids are not contiguous declaration order")
    return entries


def _continuous_out_of_bounds_strategy_from_fields(
    source: JavaSource,
    fields: list[JavaField],
    description: str,
    field_name: str = "BY_ID",
) -> str:
    fields = [field for field in fields if field.name == field_name]
    if len(fields) != 1 or fields[0].initializer is None:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one initialized {description} {field_name} field"
        )
    initializer = fields[0].initializer
    if initializer.node.type != "method_invocation":
        _fail(source, initializer, f"{description} BY_ID must be a method invocation")
    name = initializer.node.child_by_field_name("name")
    arguments = initializer.node.child_by_field_name("arguments")
    if (
        name is None
        or source.text(name) != "continuous"
        or arguments is None
        or len(arguments.named_children) != 3
    ):
        _fail(source, initializer, f"{description} BY_ID must call ByIdMap.continuous")
    strategy = source.expression(arguments.named_children[-1]).canonical.rsplit(".", 1)[-1]
    return strategy


def _continuous_out_of_bounds_strategy(source: JavaSource, description: str) -> str:
    return _continuous_out_of_bounds_strategy_from_fields(
        source, source.fields, description
    )


def difficulty_out_of_bounds_strategy(source: JavaSource) -> str:
    strategy = _continuous_out_of_bounds_strategy(source, "difficulty")
    if strategy != "WRAP":
        raise GameplayIdExtractionError(f"unsupported difficulty id strategy {strategy}")
    return strategy


def scoreboard_display_slot_entries(
    source: JavaSource, expected_count: int = 19
) -> tuple[list[ScoreboardDisplaySlotEntry], str]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"display slot {name} must have two arguments")
        entries.append(
            ScoreboardDisplaySlotEntry(
                name, _integer(source, arguments[0]), _string(source, arguments[1])
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} display slots, extracted {len(entries)}"
        )
    _validate(entries, "display slot", "slot_id")
    _validate(entries, "display slot", "serialized_name")
    if [entry.slot_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("display slot ids are not contiguous declaration order")
    strategy = _continuous_out_of_bounds_strategy(source, "display slot")
    if strategy != "ZERO":
        raise GameplayIdExtractionError(f"unsupported display slot id strategy {strategy}")
    return entries, strategy


def scoreboard_team_option_entries(
    source: JavaSource,
) -> tuple[
    list[ScoreboardTeamOptionEntry],
    str,
    list[ScoreboardTeamOptionEntry],
    str,
]:
    def extract(
        enum_name: str, description: str
    ) -> tuple[list[ScoreboardTeamOptionEntry], str]:
        enum_type = _nested_enum_type(source, enum_name)
        entries = []
        for name, arguments in _nested_enum_constants(source, enum_name, description):
            if len(arguments) != 2:
                expression = arguments[0] if arguments else source.expression(enum_type.node)
                _fail(source, expression, f"{description} {name} must have two arguments")
            entries.append(
                ScoreboardTeamOptionEntry(
                    name, _integer(source, arguments[1]), _string(source, arguments[0])
                )
            )
        if len(entries) != 4:
            raise GameplayIdExtractionError(
                f"expected 4 {description} values, extracted {len(entries)}"
            )
        _validate(entries, description, "option_id")
        _validate(entries, description, "serialized_name")
        if [entry.option_id for entry in entries] != list(range(4)):
            raise GameplayIdExtractionError(
                f"{description} ids are not contiguous declaration order"
            )
        strategy = _continuous_out_of_bounds_strategy_from_fields(
            source, source.fields_for(enum_type), description
        )
        if strategy != "ZERO":
            raise GameplayIdExtractionError(
                f"unsupported {description} id strategy {strategy}"
            )
        return entries, strategy

    visibility, visibility_strategy = extract("Visibility", "team visibility")
    collision, collision_strategy = extract("CollisionRule", "team collision rule")
    return visibility, visibility_strategy, collision, collision_strategy


def chat_formatting_entries(
    source: JavaSource, expected_count: int = 22
) -> list[ChatFormattingEntry]:
    entries = []
    for ordinal, (name, arguments) in enumerate(_enum_constants(source)):
        if len(arguments) not in {3, 4}:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"chat formatting {name} has unsupported arguments")
        declared_name = _string(source, arguments[0])
        if declared_name != name:
            _fail(
                source,
                arguments[0],
                f"chat formatting {name} declares mismatched name {declared_name}",
            )
        legacy_code = _character(source, arguments[1])
        if len(arguments) == 3:
            if arguments[2].canonical != "true":
                _fail(source, arguments[2], f"chat formatting {name} must be a format")
            formatting_id = -1
            is_format = True
            color = None
        else:
            formatting_id = _signed_integer(source, arguments[2])
            is_format = False
            color = (
                None
                if arguments[3].canonical == "null"
                else _integer(source, arguments[3])
            )
        entries.append(
            ChatFormattingEntry(
                name,
                ordinal,
                name.lower(),
                legacy_code,
                formatting_id,
                is_format,
                color,
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} chat formatting values, extracted {len(entries)}"
        )
    _validate(entries, "chat formatting", "ordinal")
    _validate(entries, "chat formatting", "serialized_name")
    if [entry.ordinal for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError(
            "chat formatting ordinals are not contiguous declaration order"
        )
    color_ids = [entry.formatting_id for entry in entries if entry.color is not None]
    if color_ids != list(range(16)):
        raise GameplayIdExtractionError("chat formatting color ids are not 0..15")
    return entries


def scoreboard_method_entries(
    objective_source: JavaSource, team_source: JavaSource
) -> tuple[list[ScoreboardMethodEntry], list[ScoreboardMethodEntry]]:
    def extract(
        source: JavaSource, description: str, expected_names: list[str]
    ) -> list[ScoreboardMethodEntry]:
        entries = []
        for field in source.fields:
            if not field.name.startswith("METHOD_"):
                continue
            if field.initializer is None:
                raise GameplayIdExtractionError(
                    f"{source.relative_path}: {field.name} lacks an initializer"
                )
            entries.append(
                ScoreboardMethodEntry(
                    field.name.removeprefix("METHOD_"),
                    _integer(source, field.initializer),
                )
            )
        if [entry.constant_name for entry in entries] != expected_names:
            raise GameplayIdExtractionError(
                f"{description} methods differ from expected declaration order: "
                f"{[entry.constant_name for entry in entries]}"
            )
        if [entry.method_id for entry in entries] != list(range(len(expected_names))):
            raise GameplayIdExtractionError(
                f"{description} method ids are not contiguous declaration order"
            )
        _validate(entries, description, "method_id")
        return entries

    objective = extract(
        objective_source, "objective", ["ADD", "REMOVE", "CHANGE"]
    )
    team = extract(
        team_source,
        "player team",
        ["ADD", "REMOVE", "CHANGE", "JOIN", "LEAVE"],
    )
    return objective, team


def boss_event_operation_type_entries(
    source: JavaSource, expected_count: int = 6
) -> list[BossEventOperationTypeEntry]:
    expected_names = [
        "ADD",
        "REMOVE",
        "UPDATE_PROGRESS",
        "UPDATE_NAME",
        "UPDATE_STYLE",
        "UPDATE_PROPERTIES",
    ]
    entries = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(source, "OperationType", "boss event operation type")
    ):
        if len(arguments) != 1:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(
                source,
                expression,
                f"boss event operation type {name} must have one reader argument",
            )
        entries.append(BossEventOperationTypeEntry(name, ordinal))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} boss event operation types, extracted {len(entries)}"
        )
    if [entry.constant_name for entry in entries] != expected_names:
        raise GameplayIdExtractionError(
            "boss event operation types differ from expected declaration order"
        )
    _validate(entries, "boss event operation type", "ordinal")
    return entries


def filter_mask_type_entries(
    source: JavaSource, expected_count: int = 3
) -> list[FilterMaskTypeEntry]:
    expected_names = ["PASS_THROUGH", "FULLY_FILTERED", "PARTIALLY_FILTERED"]
    entries = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(source, "Type", "filter mask type")
    ):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"filter mask type {name} must have two arguments")
        entries.append(FilterMaskTypeEntry(name, ordinal, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} filter mask types, extracted {len(entries)}"
        )
    if [entry.constant_name for entry in entries] != expected_names:
        raise GameplayIdExtractionError(
            "filter mask types differ from expected declaration order"
        )
    for entry in entries:
        if entry.serialized_name != entry.constant_name.lower():
            raise GameplayIdExtractionError(
                f"filter mask type {entry.constant_name} has unexpected serialized name"
            )
    _validate(entries, "filter mask type", "ordinal")
    _validate(entries, "filter mask type", "serialized_name")
    return entries


def waypoint_wire_type_entries(
    packet_source: JavaSource, waypoint_source: JavaSource
) -> tuple[list[WaypointOperationEntry], str, list[WaypointDataTypeEntry]]:
    operation_type = _nested_enum_type(packet_source, "Operation")
    operation_names = ["TRACK", "UNTRACK", "UPDATE"]
    operations = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(packet_source, "Operation", "waypoint operation")
    ):
        if len(arguments) != 1:
            expression = (
                arguments[0] if arguments else packet_source.expression(operation_type.node)
            )
            _fail(packet_source, expression, f"waypoint operation {name} needs one action")
        operations.append(WaypointOperationEntry(name, ordinal, name.lower()))
    if [entry.constant_name for entry in operations] != operation_names:
        raise GameplayIdExtractionError(
            "waypoint operations differ from expected declaration order"
        )
    operation_strategy = _continuous_out_of_bounds_strategy_from_fields(
        packet_source, packet_source.fields_for(operation_type), "waypoint operation"
    )
    if operation_strategy != "WRAP":
        raise GameplayIdExtractionError(
            f"unsupported waypoint operation id strategy {operation_strategy}"
        )
    _validate(operations, "waypoint operation", "ordinal")
    _validate(operations, "waypoint operation", "serialized_name")

    data_type = _nested_enum_type(waypoint_source, "Type")
    data_names = ["EMPTY", "VEC3I", "CHUNK", "AZIMUTH"]
    data_types = []
    for ordinal, (name, arguments) in enumerate(
        _nested_enum_constants(waypoint_source, "Type", "waypoint data type")
    ):
        if len(arguments) != 1:
            expression = (
                arguments[0] if arguments else waypoint_source.expression(data_type.node)
            )
            _fail(waypoint_source, expression, f"waypoint data type {name} needs one constructor")
        data_types.append(WaypointDataTypeEntry(name, ordinal))
    if [entry.constant_name for entry in data_types] != data_names:
        raise GameplayIdExtractionError(
            "waypoint data types differ from expected declaration order"
        )
    _validate(data_types, "waypoint data type", "ordinal")
    return operations, operation_strategy, data_types


def interaction_hand_entries(
    source: JavaSource, expected_count: int = 2
) -> tuple[list[InteractionHandEntry], str]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 1:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"interaction hand {name} must have one id argument")
        entries.append(InteractionHandEntry(name, _integer(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} interaction hands, extracted {len(entries)}"
        )
    if [entry.constant_name for entry in entries] != ["MAIN_HAND", "OFF_HAND"]:
        raise GameplayIdExtractionError(
            "interaction hands differ from expected declaration order"
        )
    if [entry.hand_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError(
            "interaction hand ids are not contiguous declaration order"
        )
    strategy = _continuous_out_of_bounds_strategy(source, "interaction hand")
    if strategy != "ZERO":
        raise GameplayIdExtractionError(
            f"unsupported interaction hand id strategy {strategy}"
        )
    _validate(entries, "interaction hand", "hand_id")
    return entries, strategy


def client_information_option_entries(
    chat_visibility_source: JavaSource,
    main_hand_source: JavaSource,
    particle_status_source: JavaSource,
) -> tuple[
    list[ClientInformationOptionEntry],
    list[ClientInformationOptionEntry],
    list[ClientInformationOptionEntry],
]:
    def extract(
        source: JavaSource,
        description: str,
        expected_names: list[str],
        argument_count: int,
    ) -> list[ClientInformationOptionEntry]:
        entries = []
        for name, arguments in _enum_constants(source):
            if len(arguments) != argument_count:
                expression = (
                    arguments[0] if arguments else source.expression(source.types[0].node)
                )
                _fail(
                    source,
                    expression,
                    f"{description} {name} must have {argument_count} arguments",
                )
            entries.append(
                ClientInformationOptionEntry(name, _integer(source, arguments[0]))
            )
        if [entry.constant_name for entry in entries] != expected_names:
            raise GameplayIdExtractionError(
                f"{description} values differ from expected declaration order"
            )
        if [entry.option_id for entry in entries] != list(range(len(expected_names))):
            raise GameplayIdExtractionError(
                f"{description} ids are not contiguous declaration order"
            )
        _validate(entries, description, "option_id")
        return entries

    return (
        extract(
            chat_visibility_source,
            "client chat visibility",
            ["FULL", "SYSTEM", "HIDDEN"],
            2,
        ),
        extract(
            main_hand_source,
            "client main hand",
            ["LEFT", "RIGHT"],
            3,
        ),
        extract(
            particle_status_source,
            "client particle status",
            ["ALL", "DECREASED", "MINIMAL"],
            2,
        ),
    )


def container_input_entries(
    source: JavaSource, expected_count: int = 7
) -> tuple[list[ContainerInputEntry], str]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 1:
            expression = arguments[0] if arguments else source.expression(source.types[0].node)
            _fail(source, expression, f"container input {name} must have one id argument")
        entries.append(ContainerInputEntry(name, _integer(source, arguments[0])))
    expected_names = [
        "PICKUP",
        "QUICK_MOVE",
        "SWAP",
        "CLONE",
        "THROW",
        "QUICK_CRAFT",
        "PICKUP_ALL",
    ]
    if (
        len(entries) != expected_count
        or [entry.constant_name for entry in entries] != expected_names
    ):
        raise GameplayIdExtractionError(
            "container inputs differ from expected declaration order"
        )
    if [entry.input_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError(
            "container input ids are not contiguous declaration order"
        )
    strategy = _continuous_out_of_bounds_strategy(source, "container input")
    if strategy != "ZERO":
        raise GameplayIdExtractionError(
            f"unsupported container input id strategy {strategy}"
        )
    _validate(entries, "container input", "input_id")
    return entries, strategy


def recipe_wire_type_entries(
    book_type_source: JavaSource,
    display_types_source: JavaSource,
    category_source: JavaSource,
    search_category_source: JavaSource,
) -> tuple[
    list[RecipeBookTypeEntry],
    list[RecipeDisplayTypeEntry],
    list[RecipeBookCategoryEntry],
    list[RecipeBookSearchCategoryEntry],
]:
    book_names = ["CRAFTING", "FURNACE", "BLAST_FURNACE", "SMOKER"]
    books = []
    for ordinal, (name, arguments) in enumerate(_enum_constants(book_type_source)):
        if arguments:
            _fail(
                book_type_source,
                arguments[0],
                f"recipe book type {name} must not have constructor arguments",
            )
        books.append(RecipeBookTypeEntry(name, ordinal))
    if [entry.constant_name for entry in books] != book_names:
        raise GameplayIdExtractionError(
            "recipe book types differ from expected declaration order"
        )
    _validate(books, "recipe book type", "ordinal")

    top_types = [
        java_type
        for java_type in display_types_source.types
        if java_type.node.parent.type == "program"
        and java_type.simple_name == "RecipeDisplays"
    ]
    if len(top_types) != 1:
        raise GameplayIdExtractionError(
            f"{display_types_source.relative_path}: expected one RecipeDisplays type"
        )
    register_calls = [
        call
        for call in display_types_source.calls_for(top_types[0])
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "Registry"
    ]
    displays = []
    expected_type_refs = [
        "ShapelessCraftingRecipeDisplay.TYPE",
        "ShapedCraftingRecipeDisplay.TYPE",
        "FurnaceRecipeDisplay.TYPE",
        "StonecutterRecipeDisplay.TYPE",
        "SmithingRecipeDisplay.TYPE",
    ]
    for type_id, call in enumerate(register_calls):
        if len(call.arguments) != 3:
            expression = display_types_source.expression(call.node)
            _fail(
                display_types_source,
                expression,
                "recipe display register call must have three arguments",
            )
        if call.arguments[0].canonical != "registry":
            _fail(
                display_types_source,
                call.arguments[0],
                "recipe display registration must target the bootstrap registry",
            )
        resource_path = _string(display_types_source, call.arguments[1])
        type_ref = call.arguments[2].canonical
        if type_id >= len(expected_type_refs) or type_ref != expected_type_refs[type_id]:
            _fail(
                display_types_source,
                call.arguments[2],
                f"unexpected recipe display type reference {type_ref}",
            )
        displays.append(
            RecipeDisplayTypeEntry(
                resource_path.upper(), type_id, "minecraft:" + resource_path
            )
        )
    if len(displays) != len(expected_type_refs):
        raise GameplayIdExtractionError(
            f"expected {len(expected_type_refs)} recipe display registrations, "
            f"extracted {len(displays)}"
        )
    _validate(displays, "recipe display type", "type_id")
    _validate(displays, "recipe display type", "resource_id")

    category_names = [
        "CRAFTING_BUILDING_BLOCKS",
        "CRAFTING_REDSTONE",
        "CRAFTING_EQUIPMENT",
        "CRAFTING_MISC",
        "FURNACE_FOOD",
        "FURNACE_BLOCKS",
        "FURNACE_MISC",
        "BLAST_FURNACE_BLOCKS",
        "BLAST_FURNACE_MISC",
        "SMOKER_FOOD",
        "STONECUTTER",
        "SMITHING",
        "CAMPFIRE",
    ]
    categories = []
    for category_id, field in enumerate(_fields(category_source, "RecipeBookCategory")):
        arguments = _register_arguments(category_source, field)
        if len(arguments) != 1:
            _fail(
                category_source,
                field.initializer,
                f"recipe book category {field.name} must register one resource id",
            )
        resource_path = _string(category_source, arguments[0])
        categories.append(
            RecipeBookCategoryEntry(
                field.name, category_id, "minecraft:" + resource_path
            )
        )
    if [entry.constant_name for entry in categories] != category_names:
        raise GameplayIdExtractionError(
            "recipe book categories differ from expected registration order"
        )
    _validate(categories, "recipe book category", "category_id")
    _validate(categories, "recipe book category", "resource_id")

    known_categories = {entry.constant_name for entry in categories}
    search_categories = []
    for name, arguments in _enum_constants(search_category_source):
        included = []
        for argument in arguments:
            node = argument.node
            object_node = node.child_by_field_name("object")
            field_node = node.child_by_field_name("field")
            if (
                node.type != "field_access"
                or object_node is None
                or field_node is None
                or search_category_source.expression(object_node).canonical
                != "RecipeBookCategories"
            ):
                _fail(
                    search_category_source,
                    argument,
                    "search recipe book category must reference RecipeBookCategories",
                )
            category_name = search_category_source.text(field_node)
            if category_name not in known_categories:
                _fail(
                    search_category_source,
                    argument,
                    f"unknown search recipe book category {category_name}",
                )
            included.append(category_name)
        if not included or len(included) != len(set(included)):
            expression = arguments[0] if arguments else search_category_source.expression(
                search_category_source.types[0].node
            )
            _fail(
                search_category_source,
                expression,
                f"search recipe book category {name} must contain unique categories",
            )
        search_categories.append(RecipeBookSearchCategoryEntry(name, tuple(included)))
    if [entry.constant_name for entry in search_categories] != book_names:
        raise GameplayIdExtractionError(
            "search recipe book categories differ from recipe book type order"
        )
    if [len(entry.included_category_constants) for entry in search_categories] != [
        4,
        3,
        2,
        1,
    ]:
        raise GameplayIdExtractionError(
            "search recipe book category membership counts differ from vanilla"
        )
    _validate(search_categories, "search recipe book category")
    return books, displays, categories, search_categories


def slot_display_type_entries(source: JavaSource) -> list[SlotDisplayTypeEntry]:
    top_types = [
        java_type
        for java_type in source.types
        if java_type.node.parent.type == "program"
        and java_type.simple_name == "SlotDisplays"
    ]
    if len(top_types) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one SlotDisplays type"
        )
    register_calls = [
        call
        for call in source.calls_for(top_types[0])
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "Registry"
    ]
    expected_type_refs = [
        "SlotDisplay.Empty.TYPE",
        "SlotDisplay.AnyFuel.TYPE",
        "SlotDisplay.WithAnyPotion.TYPE",
        "SlotDisplay.OnlyWithComponent.TYPE",
        "SlotDisplay.ItemSlotDisplay.TYPE",
        "SlotDisplay.ItemStackSlotDisplay.TYPE",
        "SlotDisplay.TagSlotDisplay.TYPE",
        "SlotDisplay.DyedSlotDemo.TYPE",
        "SlotDisplay.SmithingTrimDemoSlotDisplay.TYPE",
        "SlotDisplay.WithRemainder.TYPE",
        "SlotDisplay.Composite.TYPE",
    ]
    entries = []
    for type_id, call in enumerate(register_calls):
        if len(call.arguments) != 3:
            _fail(
                source,
                source.expression(call.node),
                "slot display register call must have three arguments",
            )
        if call.arguments[0].canonical != "registry":
            _fail(
                source,
                call.arguments[0],
                "slot display registration must target the bootstrap registry",
            )
        resource_path = _string(source, call.arguments[1])
        type_ref = call.arguments[2].canonical
        if type_id >= len(expected_type_refs) or type_ref != expected_type_refs[type_id]:
            _fail(
                source,
                call.arguments[2],
                f"unexpected slot display type reference {type_ref}",
            )
        entries.append(
            SlotDisplayTypeEntry(
                resource_path.upper(), type_id, "minecraft:" + resource_path
            )
        )
    if len(entries) != len(expected_type_refs):
        raise GameplayIdExtractionError(
            f"expected {len(expected_type_refs)} slot display registrations, "
            f"extracted {len(entries)}"
        )
    _validate(entries, "slot display type", "type_id")
    _validate(entries, "slot display type", "resource_id")
    return entries


def item_component_value_type_entries(
    item_use_source: JavaSource,
    swing_source: JavaSource,
    rarity_source: JavaSource,
    map_post_processing_source: JavaSource,
    firework_source: JavaSource,
) -> tuple[
    list[NamedComponentValueEntry],
    list[NamedComponentValueEntry],
    list[NamedComponentValueEntry],
    list[ComponentValueEntry],
    list[NamedComponentValueEntry],
]:
    def named(
        source: JavaSource,
        description: str,
        expected_names: list[str],
        allowed_argument_counts: set[int],
        constants: list[tuple[str, tuple[JavaExpression, ...]]] | None = None,
    ) -> list[NamedComponentValueEntry]:
        entries = []
        for name, arguments in (
            constants if constants is not None else _enum_constants(source)
        ):
            if len(arguments) not in allowed_argument_counts:
                expression = (
                    arguments[0] if arguments else source.expression(source.types[0].node)
                )
                _fail(source, expression, f"{description} {name} has unexpected arguments")
            entries.append(
                NamedComponentValueEntry(
                    name,
                    _integer(source, arguments[0]),
                    _string(source, arguments[1]),
                )
            )
        if [entry.constant_name for entry in entries] != expected_names:
            raise GameplayIdExtractionError(
                f"{description} values differ from expected declaration order"
            )
        if [entry.value_id for entry in entries] != list(range(len(expected_names))):
            raise GameplayIdExtractionError(
                f"{description} ids are not contiguous declaration order"
            )
        for entry in entries:
            if entry.serialized_name != entry.constant_name.lower():
                raise GameplayIdExtractionError(
                    f"{description} {entry.constant_name} has unexpected serialized name"
                )
        _validate(entries, description, "value_id")
        _validate(entries, description, "serialized_name")
        return entries

    item_uses = named(
        item_use_source,
        "item use animation",
        [
            "NONE",
            "EAT",
            "DRINK",
            "BLOCK",
            "BOW",
            "TRIDENT",
            "CROSSBOW",
            "SPYGLASS",
            "TOOT_HORN",
            "BRUSH",
            "BUNDLE",
            "SPEAR",
        ],
        {2, 3},
    )
    swings = named(
        swing_source,
        "swing animation type",
        ["NONE", "WHACK", "STAB"],
        {2},
    )
    rarities = named(
        rarity_source,
        "item rarity",
        ["COMMON", "UNCOMMON", "RARE", "EPIC"],
        {3},
    )

    map_values = []
    for name, arguments in _enum_constants(map_post_processing_source):
        if len(arguments) != 1:
            expression = (
                arguments[0]
                if arguments
                else map_post_processing_source.expression(
                    map_post_processing_source.types[0].node
                )
            )
            _fail(
                map_post_processing_source,
                expression,
                f"map post-processing {name} must have one id argument",
            )
        map_values.append(
            ComponentValueEntry(
                name, _integer(map_post_processing_source, arguments[0])
            )
        )
    if [entry.constant_name for entry in map_values] != ["LOCK", "SCALE"]:
        raise GameplayIdExtractionError(
            "map post-processing values differ from expected declaration order"
        )
    if [entry.value_id for entry in map_values] != [0, 1]:
        raise GameplayIdExtractionError(
            "map post-processing ids are not contiguous declaration order"
        )
    _validate(map_values, "map post-processing", "value_id")

    firework_type = _nested_enum_type(firework_source, "Shape")
    firework_shapes = named(
        firework_source,
        "firework explosion shape",
        ["SMALL_BALL", "LARGE_BALL", "STAR", "CREEPER", "BURST"],
        {2},
        _nested_enum_constants(
            firework_source, "Shape", "firework explosion shape"
        ),
    )

    strategies = [
        _continuous_out_of_bounds_strategy(item_use_source, "item use animation"),
        _continuous_out_of_bounds_strategy(swing_source, "swing animation type"),
        _continuous_out_of_bounds_strategy(rarity_source, "item rarity"),
        _continuous_out_of_bounds_strategy_from_fields(
            map_post_processing_source,
            map_post_processing_source.fields,
            "map post-processing",
            "ID_MAP",
        ),
        _continuous_out_of_bounds_strategy_from_fields(
            firework_source,
            firework_source.fields_for(firework_type),
            "firework explosion shape",
        ),
    ]
    if any(strategy != "ZERO" for strategy in strategies):
        raise GameplayIdExtractionError(
            f"unsupported item component value id strategies {strategies}"
        )
    return item_uses, swings, rarities, map_values, firework_shapes


def builtin_enum_component_value_entries(
    data_components_source: JavaSource,
    dye_color_source: JavaSource,
    fox_source: JavaSource,
    salmon_source: JavaSource,
    parrot_source: JavaSource,
    mushroom_cow_source: JavaSource,
    rabbit_source: JavaSource,
    horse_variant_source: JavaSource,
    llama_source: JavaSource,
    axolotl_source: JavaSource,
) -> tuple[list[BuiltinEnumValueTypeEntry], list[BuiltinEnumComponentEntry]]:
    def extract(
        source: JavaSource,
        constant_name: str,
        expected_names: list[str],
        id_argument: int,
        name_argument: int,
        nested_enum: str | None = None,
        sparse_default: str | None = None,
        texture_diffuse_argument: int | None = None,
    ) -> BuiltinEnumValueTypeEntry:
        enum_type = (
            _nested_enum_type(source, nested_enum)
            if nested_enum is not None
            else source.types[0]
        )
        constants = (
            _nested_enum_constants(source, nested_enum, constant_name)
            if nested_enum is not None
            else _enum_constants(source)
        )
        values = []
        texture_diffuse_colors = []
        for name, arguments in constants:
            required_arguments = max(
                id_argument,
                name_argument,
                texture_diffuse_argument
                if texture_diffuse_argument is not None
                else 0,
            ) + 1
            if len(arguments) < required_arguments:
                expression = arguments[0] if arguments else source.expression(enum_type.node)
                _fail(source, expression, f"{constant_name} {name} lacks id/name arguments")
            values.append(
                NamedComponentValueEntry(
                    name,
                    _integer(source, arguments[id_argument]),
                    _string(source, arguments[name_argument]),
                )
            )
            if texture_diffuse_argument is not None:
                texture_diffuse_color = _integer(
                    source, arguments[texture_diffuse_argument]
                )
                if not 0 <= texture_diffuse_color <= 0xFF_FFFF:
                    _fail(
                        source,
                        arguments[texture_diffuse_argument],
                        f"{constant_name} {name} texture diffuse color is not RGB",
                    )
                texture_diffuse_colors.append(texture_diffuse_color)
        if [value.constant_name for value in values] != expected_names:
            raise GameplayIdExtractionError(
                f"{constant_name} values differ from expected declaration order"
            )
        _validate(values, constant_name, "value_id")
        _validate(values, constant_name, "serialized_name")

        if sparse_default is None:
            strategy = _continuous_out_of_bounds_strategy_from_fields(
                source, source.fields_for(enum_type), constant_name
            )
            if strategy not in {"ZERO", "CLAMP", "WRAP"}:
                raise GameplayIdExtractionError(
                    f"unsupported {constant_name} id strategy {strategy}"
                )
            if [value.value_id for value in values] != list(range(len(values))):
                raise GameplayIdExtractionError(
                    f"{constant_name} ids are not contiguous declaration order"
                )
            default_id = values[0].value_id
        else:
            by_id = [
                field
                for field in source.fields_for(enum_type)
                if field.name == "BY_ID" and field.initializer is not None
            ]
            defaults = [
                field
                for field in source.fields_for(enum_type)
                if field.name == "DEFAULT" and field.initializer is not None
            ]
            if len(by_id) != 1 or len(defaults) != 1:
                raise GameplayIdExtractionError(
                    f"{source.relative_path}: expected initialized {constant_name} BY_ID and DEFAULT"
                )
            initializer = by_id[0].initializer
            name = initializer.node.child_by_field_name("name")
            arguments = initializer.node.child_by_field_name("arguments")
            if (
                initializer.node.type != "method_invocation"
                or name is None
                or source.text(name) != "sparse"
                or arguments is None
                or len(arguments.named_children) != 3
                or source.expression(arguments.named_children[-1]).canonical != "DEFAULT"
                or defaults[0].initializer.canonical != sparse_default
            ):
                _fail(source, initializer, f"{constant_name} must use sparse DEFAULT lookup")
            strategy = "SPARSE"
            default_id = next(
                value.value_id
                for value in values
                if value.constant_name == sparse_default
            )
        return BuiltinEnumValueTypeEntry(
            constant_name,
            strategy,
            default_id,
            tuple(values),
            tuple(texture_diffuse_colors)
            if texture_diffuse_argument is not None
            else None,
        )

    value_types = [
        extract(
            dye_color_source,
            "DYE_COLOR",
            [
                "WHITE", "ORANGE", "MAGENTA", "LIGHT_BLUE", "YELLOW", "LIME",
                "PINK", "GRAY", "LIGHT_GRAY", "CYAN", "PURPLE", "BLUE", "BROWN",
                "GREEN", "RED", "BLACK",
            ],
            0,
            1,
            texture_diffuse_argument=2,
        ),
        extract(fox_source, "FOX_VARIANT", ["RED", "SNOW"], 0, 1, "Variant"),
        extract(salmon_source, "SALMON_VARIANT", ["SMALL", "MEDIUM", "LARGE"], 1, 0, "Variant"),
        extract(
            parrot_source,
            "PARROT_VARIANT",
            ["RED_BLUE", "BLUE", "GREEN", "YELLOW_BLUE", "GRAY"],
            0,
            1,
            "Variant",
        ),
        extract(mushroom_cow_source, "MOOSHROOM_VARIANT", ["RED", "BROWN"], 1, 0, "Variant"),
        extract(
            rabbit_source,
            "RABBIT_VARIANT",
            ["BROWN", "WHITE", "BLACK", "WHITE_SPLOTCHED", "GOLD", "SALT", "EVIL"],
            0,
            1,
            "Variant",
            "BROWN",
        ),
        extract(
            horse_variant_source,
            "HORSE_VARIANT",
            ["WHITE", "CREAMY", "CHESTNUT", "BROWN", "BLACK", "GRAY", "DARK_BROWN"],
            0,
            1,
        ),
        extract(llama_source, "LLAMA_VARIANT", ["CREAMY", "WHITE", "BROWN", "GRAY"], 0, 1, "Variant"),
        extract(
            axolotl_source,
            "AXOLOTL_VARIANT",
            ["LUCY", "WILD", "GOLD", "CYAN", "BLUE"],
            0,
            1,
            "Variant",
        ),
    ]

    java_type_to_value_type = {
        "DyeColor": "DYE_COLOR",
        "Fox.Variant": "FOX_VARIANT",
        "Salmon.Variant": "SALMON_VARIANT",
        "Parrot.Variant": "PARROT_VARIANT",
        "MushroomCow.Variant": "MOOSHROOM_VARIANT",
        "Rabbit.Variant": "RABBIT_VARIANT",
        "Variant": "HORSE_VARIANT",
        "Llama.Variant": "LLAMA_VARIANT",
        "Axolotl.Variant": "AXOLOTL_VARIANT",
    }
    expected_components = [
        "DYE",
        "BASE_COLOR",
        "WOLF_COLLAR",
        "FOX_VARIANT",
        "SALMON_SIZE",
        "PARROT_VARIANT",
        "TROPICAL_FISH_BASE_COLOR",
        "TROPICAL_FISH_PATTERN_COLOR",
        "MOOSHROOM_VARIANT",
        "RABBIT_VARIANT",
        "HORSE_VARIANT",
        "LLAMA_VARIANT",
        "AXOLOTL_VARIANT",
        "CAT_COLLAR",
        "SHEEP_COLOR",
        "SHULKER_COLOR",
    ]
    fields = []
    for field in data_components_source.fields:
        type_spelling = field.type_spelling.replace(" ", "")
        if not type_spelling.startswith("DataComponentType<") or not type_spelling.endswith(">"):
            continue
        java_type = type_spelling.removeprefix("DataComponentType<").removesuffix(">")
        if java_type in java_type_to_value_type:
            fields.append(field)
    if [field.name for field in fields] != expected_components:
        raise GameplayIdExtractionError(
            "built-in enum data components differ from expected declaration order"
        )
    components = []
    for field in fields:
        type_spelling = field.type_spelling.replace(" ", "")
        if not type_spelling.startswith("DataComponentType<") or not type_spelling.endswith(">"):
            raise GameplayIdExtractionError(
                f"{data_components_source.relative_path}: {field.name} has unexpected component type"
            )
        java_type = type_spelling.removeprefix("DataComponentType<").removesuffix(">")
        value_type = java_type_to_value_type.get(java_type)
        if value_type is None or field.initializer is None:
            raise GameplayIdExtractionError(
                f"{data_components_source.relative_path}: {field.name} has unsupported value type {java_type}"
            )
        calls = [
            call
            for call in data_components_source.calls
            if field.initializer.span.start_byte <= call.span.start_byte
            and call.span.end_byte <= field.initializer.span.end_byte
        ]
        codec_owner = java_type
        persistent = [call for call in calls if call.name == "persistent"]
        network = [call for call in calls if call.name == "networkSynchronized"]
        if (
            len(persistent) != 1
            or len(network) != 1
            or [arg.canonical for arg in persistent[0].arguments] != [codec_owner + ".CODEC"]
            or [arg.canonical for arg in network[0].arguments] != [codec_owner + ".STREAM_CODEC"]
        ):
            _fail(
                data_components_source,
                field.initializer,
                f"{field.name} codec owner does not match {java_type}",
            )
        components.append(BuiltinEnumComponentEntry(field.name, value_type))
    _validate(components, "built-in enum component", "constant_name")
    return value_types, components


def consume_effect_type_entries(
    source: JavaSource, expected_count: int = 5
) -> list[ConsumeEffectTypeEntry]:
    types = [
        java_type
        for java_type in source.types
        if java_type.kind == "record"
        and java_type.simple_name == "Type"
        and java_type.fqcn.endswith(".ConsumeEffect.Type")
    ]
    if len(types) != 1:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one nested ConsumeEffect.Type record"
        )
    fields = [
        field
        for field in source.fields_for(types[0])
        if field.type_spelling.replace(" ", "").startswith("ConsumeEffect.Type<")
    ]
    entries = []
    for type_id, field in enumerate(fields):
        arguments = _register_arguments(source, field)
        if len(arguments) != 3:
            _fail(
                source,
                field.initializer,
                f"consume effect type {field.name} register must have three arguments",
            )
        codec_owner = arguments[1].canonical.removesuffix(".CODEC")
        stream_owner = arguments[2].canonical.removesuffix(".STREAM_CODEC")
        if (
            codec_owner == arguments[1].canonical
            or stream_owner == arguments[2].canonical
            or codec_owner != stream_owner
        ):
            _fail(
                source,
                arguments[1],
                f"consume effect type {field.name} codec references disagree",
            )
        resource_path = _string(source, arguments[0])
        entries.append(
            ConsumeEffectTypeEntry(
                field.name, type_id, "minecraft:" + resource_path
            )
        )
    expected_names = [
        "APPLY_EFFECTS",
        "REMOVE_EFFECTS",
        "CLEAR_ALL_EFFECTS",
        "TELEPORT_RANDOMLY",
        "PLAY_SOUND",
    ]
    if (
        len(entries) != expected_count
        or [entry.constant_name for entry in entries] != expected_names
    ):
        raise GameplayIdExtractionError(
            "consume effect types differ from expected declaration order"
        )
    if [entry.type_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError(
            "consume effect type ids are not contiguous declaration order"
        )
    for entry in entries:
        if entry.resource_id.removeprefix("minecraft:") != entry.constant_name.lower():
            raise GameplayIdExtractionError(
                f"consume effect type {entry.constant_name} has unexpected resource id"
            )
    _validate(entries, "consume effect type", "type_id")
    _validate(entries, "consume effect type", "resource_id")
    return entries


def attribute_modifier_type_entries(
    modifier_source: JavaSource,
    item_modifiers_source: JavaSource,
) -> tuple[list[NamedComponentValueEntry], list[NamedComponentValueEntry]]:
    operation_type = _nested_enum_type(modifier_source, "Operation")
    operations = []
    for name, arguments in _nested_enum_constants(
        modifier_source, "Operation", "attribute modifier operation"
    ):
        if len(arguments) != 2:
            expression = arguments[0] if arguments else modifier_source.expression(operation_type.node)
            _fail(modifier_source, expression, f"attribute modifier operation {name} must have two arguments")
        operations.append(
            NamedComponentValueEntry(
                name,
                _integer(modifier_source, arguments[1]),
                _string(modifier_source, arguments[0]),
            )
        )
    expected_operations = ["ADD_VALUE", "ADD_MULTIPLIED_BASE", "ADD_MULTIPLIED_TOTAL"]
    if [entry.constant_name for entry in operations] != expected_operations:
        raise GameplayIdExtractionError(
            "attribute modifier operations differ from expected declaration order"
        )
    if [entry.value_id for entry in operations] != list(range(len(operations))):
        raise GameplayIdExtractionError(
            "attribute modifier operation ids are not contiguous declaration order"
        )
    operation_strategy = _continuous_out_of_bounds_strategy_from_fields(
        modifier_source,
        modifier_source.fields_for(operation_type),
        "attribute modifier operation",
    )

    display_type = _nested_enum_type(item_modifiers_source, "Type")
    displays = []
    expected_codec_owners = ["Default", "Hidden", "OverrideText"]
    display_constants = _nested_enum_constants(
        item_modifiers_source, "Type", "attribute modifier display type"
    )
    for index, (name, arguments) in enumerate(display_constants):
        if index >= len(expected_codec_owners):
            raise GameplayIdExtractionError(
                "attribute modifier display types contain an unexpected extra entry"
            )
        if len(arguments) != 4:
            expression = arguments[0] if arguments else item_modifiers_source.expression(display_type.node)
            _fail(item_modifiers_source, expression, f"attribute modifier display type {name} must have four arguments")
        owner = f"ItemAttributeModifiers.Display.{expected_codec_owners[index]}"
        if arguments[2].canonical != owner + ".CODEC" or arguments[3].canonical != owner + ".STREAM_CODEC":
            _fail(
                item_modifiers_source,
                arguments[2],
                f"attribute modifier display type {name} codec owner differs from {owner}",
            )
        displays.append(
            NamedComponentValueEntry(
                name,
                _integer(item_modifiers_source, arguments[1]),
                _string(item_modifiers_source, arguments[0]),
            )
        )
    expected_displays = ["DEFAULT", "HIDDEN", "OVERRIDE"]
    if [entry.constant_name for entry in displays] != expected_displays:
        raise GameplayIdExtractionError(
            "attribute modifier display types differ from expected declaration order"
        )
    if [entry.value_id for entry in displays] != list(range(len(displays))):
        raise GameplayIdExtractionError(
            "attribute modifier display type ids are not contiguous declaration order"
        )
    display_strategy = _continuous_out_of_bounds_strategy_from_fields(
        item_modifiers_source,
        item_modifiers_source.fields_for(display_type),
        "attribute modifier display type",
    )
    if operation_strategy != "ZERO" or display_strategy != "ZERO":
        raise GameplayIdExtractionError(
            f"unsupported attribute modifier id strategies {operation_strategy}/{display_strategy}"
        )
    _validate(operations, "attribute modifier operation", "value_id")
    _validate(operations, "attribute modifier operation", "serialized_name")
    _validate(displays, "attribute modifier display type", "value_id")
    _validate(displays, "attribute modifier display type", "serialized_name")
    return operations, displays


def tropical_fish_pattern_entries(
    source: JavaSource,
    dye_color_source: JavaSource,
) -> tuple[list[TropicalFishPatternEntry], list[TropicalFishCommonVariantEntry]]:
    base_constants = _nested_enum_constants(source, "Base", "tropical fish base")
    bases = {}
    for name, arguments in base_constants:
        if len(arguments) != 1:
            _fail(source, arguments[0], f"tropical fish base {name} must have one id")
        bases[name] = _integer(source, arguments[0])
    if bases != {"SMALL": 0, "LARGE": 1}:
        raise GameplayIdExtractionError("tropical fish bases differ from SMALL=0/LARGE=1")

    pattern_type = _nested_enum_type(source, "Pattern")
    entries = []
    for name, arguments in _nested_enum_constants(source, "Pattern", "tropical fish pattern"):
        if len(arguments) != 3:
            expression = arguments[0] if arguments else source.expression(pattern_type.node)
            _fail(source, expression, f"tropical fish pattern {name} must have three arguments")
        base_name = arguments[1].canonical.rsplit(".", 1)[-1]
        if base_name not in bases:
            _fail(source, arguments[1], f"tropical fish pattern {name} has unknown base")
        index = _integer(source, arguments[2])
        entries.append(
            TropicalFishPatternEntry(
                name,
                _string(source, arguments[0]),
                bases[base_name],
                index,
                bases[base_name] | index << 8,
            )
        )
    expected = [
        "KOB", "SUNSTREAK", "SNOOPER", "DASHER", "BRINELY", "SPOTTY",
        "FLOPPER", "STRIPEY", "GLITTER", "BLOCKFISH", "BETTY", "CLAYFISH",
    ]
    if [entry.constant_name for entry in entries] != expected:
        raise GameplayIdExtractionError("tropical fish patterns differ from expected order")
    if [entry.pattern_index for entry in entries] != list(range(6)) * 2:
        raise GameplayIdExtractionError("tropical fish pattern indices differ from 0..5 per base")
    by_id = [
        field for field in source.fields_for(pattern_type)
        if field.name == "BY_ID" and field.initializer is not None
    ]
    if len(by_id) != 1:
        raise GameplayIdExtractionError(f"{source.relative_path}: expected one Pattern.BY_ID")
    initializer = by_id[0].initializer
    name = initializer.node.child_by_field_name("name")
    arguments = initializer.node.child_by_field_name("arguments")
    if (
        initializer.node.type != "method_invocation"
        or name is None
        or source.text(name) != "sparse"
        or arguments is None
        or len(arguments.named_children) != 3
        or source.expression(arguments.named_children[-1]).canonical != "KOB"
    ):
        _fail(source, initializer, "tropical fish Pattern.BY_ID must use sparse KOB fallback")
    _validate(entries, "tropical fish pattern", "serialized_name")
    _validate(entries, "tropical fish pattern", "packed_id")
    dye_colors = {}
    for name, arguments in _enum_constants(dye_color_source):
        if not arguments:
            _fail(
                dye_color_source,
                dye_color_source.expression(dye_color_source.types[0].node),
                f"dye color {name} lacks id",
            )
        dye_colors[name] = _integer(dye_color_source, arguments[0])
    if len(dye_colors) != 16:
        raise GameplayIdExtractionError("expected 16 dye colors for tropical fish variants")
    patterns = {entry.constant_name: entry.packed_id for entry in entries}
    common_fields = [
        field for field in source.fields
        if field.name == "COMMON_VARIANTS" and field.initializer is not None
    ]
    if len(common_fields) != 1:
        raise GameplayIdExtractionError(f"{source.relative_path}: expected one COMMON_VARIANTS field")
    common_variants = []
    for node in descendants(common_fields[0].initializer.node):
        if node.type != "object_creation_expression":
            continue
        type_node = node.child_by_field_name("type")
        arguments = node.child_by_field_name("arguments")
        if type_node is None or arguments is None:
            _fail(source, source.expression(node), "common tropical fish variant is incomplete")
        if source.expression(type_node).canonical != "TropicalFish.Variant":
            continue
        values = [source.expression(child).canonical for child in arguments.named_children]
        if len(values) != 3:
            _fail(source, source.expression(node), "common tropical fish variant must have three arguments")
        pattern_name = values[0].rsplit(".", 1)[-1]
        base_color = values[1].rsplit(".", 1)[-1]
        pattern_color = values[2].rsplit(".", 1)[-1]
        if pattern_name not in patterns or base_color not in dye_colors or pattern_color not in dye_colors:
            _fail(source, source.expression(node), "common tropical fish variant references unknown constants")
        common_variants.append(
            TropicalFishCommonVariantEntry(
                patterns[pattern_name], dye_colors[base_color], dye_colors[pattern_color]
            )
        )
    if len(common_variants) != 22:
        raise GameplayIdExtractionError(
            f"expected 22 common tropical fish variants, extracted {len(common_variants)}"
        )
    return entries, common_variants


def equipment_slot_entries(
    source: JavaSource, expected_count: int = 8
) -> list[EquipmentSlotEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) not in {4, 5}:
            _fail(source, arguments[0], f"equipment slot {name} has {len(arguments)} arguments")
        slot_type = arguments[0].canonical.rsplit(".", 1)[-1]
        numeric = [_integer(source, argument) for argument in arguments[1:-1]]
        entries.append(
            EquipmentSlotEntry(
                name,
                slot_type,
                numeric[0],
                numeric[1] if len(numeric) == 3 else 0,
                numeric[2] if len(numeric) == 3 else numeric[1],
                _string(source, arguments[-1]),
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} equipment slots, extracted {len(entries)}"
        )
    _validate(entries, "equipment slot", "slot_id")
    _validate(entries, "equipment slot", "serialized_name")
    return entries


def player_inventory_equipment_slot_indices(
    source: JavaSource, slots: list[EquipmentSlotEntry]
) -> dict[str, int]:
    def int_field(name: str) -> int:
        fields = [field for field in source.fields if field.name == name]
        if len(fields) != 1 or fields[0].initializer is None:
            raise GameplayIdExtractionError(
                f"{source.relative_path}: expected one initialized {name} field"
            )
        return _integer(source, fields[0].initializer)

    inventory_size = int_field("INVENTORY_SIZE")
    special_slots = {
        "OFFHAND": int_field("SLOT_OFFHAND"),
        "BODY": int_field("SLOT_BODY_ARMOR"),
        "SADDLE": int_field("SLOT_SADDLE"),
    }
    mapping_fields = [
        field for field in source.fields if field.name == "EQUIPMENT_SLOT_MAPPING"
    ]
    if len(mapping_fields) != 1 or mapping_fields[0].initializer is None:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: expected one initialized EQUIPMENT_SLOT_MAPPING field"
        )
    map_calls = [
        call
        for call in _calls(source, mapping_fields[0].initializer, "of")
        if call.qualifier is not None and call.qualifier.canonical == "Map"
    ]
    if len(map_calls) != 1 or len(map_calls[0].arguments) % 2 != 0:
        _fail(
            source,
            mapping_fields[0].initializer,
            "equipment slot mapping must contain one Map.of call with key/value pairs",
        )

    slot_by_name = {slot.constant_name: slot for slot in slots}
    expected_names = {"FEET", "LEGS", "CHEST", "HEAD", *special_slots}
    indices: dict[str, int] = {}
    arguments = map_calls[0].arguments
    for offset in range(0, len(arguments), 2):
        key, value = arguments[offset : offset + 2]
        value_parts = value.canonical.split(".")
        if len(value_parts) != 2 or value_parts[0] != "EquipmentSlot":
            _fail(source, value, "equipment slot mapping value must name EquipmentSlot")
        name = value_parts[1]
        if name not in expected_names or name not in slot_by_name:
            _fail(source, value, f"unexpected player inventory equipment slot {name}")
        if name in indices:
            _fail(source, value, f"duplicate player inventory equipment slot {name}")
        if name in special_slots:
            index = _integer(source, key)
            if index != special_slots[name]:
                _fail(source, key, f"{name} mapping differs from its Inventory slot constant")
        else:
            expected_key = f"EquipmentSlot.{name}.getIndex ({inventory_size})"
            if key.canonical != expected_key:
                _fail(
                    source,
                    key,
                    f"{name} mapping must be {expected_key}, found {key.canonical}",
                )
            index = inventory_size + slot_by_name[name].index
        indices[name] = index
    if set(indices) != expected_names:
        raise GameplayIdExtractionError(
            f"{source.relative_path}: player inventory equipment mapping differs: "
            f"expected {sorted(expected_names)}, extracted {sorted(indices)}"
        )
    return indices


def equipment_slot_group_entries(
    source: JavaSource, expected_count: int = 11
) -> list[EquipmentSlotGroupEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) < 3:
            _fail(source, arguments[0], f"equipment slot group {name} lacks arguments")
        entries.append(
            EquipmentSlotGroupEntry(
                name, _integer(source, arguments[0]), _string(source, arguments[1])
            )
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} equipment slot groups, extracted {len(entries)}"
        )
    _validate(entries, "equipment slot group", "group_id")
    _validate(entries, "equipment slot group", "serialized_name")
    return entries


def entity_pose_entries(source: JavaSource, expected_count: int = 18) -> list[EntityPoseEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 2:
            _fail(source, arguments[0], f"entity pose {name} must have two arguments")
        entries.append(
            EntityPoseEntry(name, _integer(source, arguments[0]), _string(source, arguments[1]))
        )
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} entity poses, extracted {len(entries)}"
        )
    _validate(entries, "entity pose", "pose_id")
    _validate(entries, "entity pose", "serialized_name")
    return entries


def entity_data_serializer_entries(
    source: JavaSource, expected_count: int = 43
) -> list[EntityDataSerializerEntry]:
    declarations = {field.name for field in _fields(source, "EntityDataSerializer")}
    calls = [call for call in source.calls if call.name == "registerSerializer"]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries = []
    for call in calls:
        if len(call.arguments) != 1 or call.arguments[0].node.type != "identifier":
            _fail(source, source.expression(call.node), "unsupported serializer registration")
        entries.append(EntityDataSerializerEntry(call.arguments[0].canonical))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} entity-data serializers, extracted {len(entries)}"
        )
    _validate(entries, "entity-data serializer")
    missing = [entry.constant_name for entry in entries if entry.constant_name not in declarations]
    if missing:
        raise GameplayIdExtractionError(f"undeclared entity-data serializers: {missing}")
    return entries


def client_game_event_type_entries(
    source: JavaSource, expected_count: int = 14
) -> list[ClientGameEventTypeEntry]:
    entries = []
    for field in _fields(source, "ClientboundGameEventPacket.Type"):
        if not field.name.isupper():
            continue
        if field.initializer is None:
            _fail(source, JavaExpression("", field.span, field.node), "game event type lacks initializer")
        arguments = _object_arguments(source, field.initializer)
        if len(arguments) != 1:
            _fail(source, field.initializer, "game event type constructor must have one argument")
        entries.append(ClientGameEventTypeEntry(field.name, _integer(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} game-event types, extracted {len(entries)}"
        )
    _validate(entries, "client game-event type", "event_id")
    if [entry.event_id for entry in entries] != list(range(expected_count)):
        raise GameplayIdExtractionError("client game-event ids are not contiguous")
    return entries


def sound_source_entries(source: JavaSource, expected_count: int = 11) -> list[SoundSourceEntry]:
    entries = []
    for name, arguments in _enum_constants(source):
        if len(arguments) != 1:
            _fail(source, arguments[0], f"sound source {name} must have one argument")
        entries.append(SoundSourceEntry(name, _string(source, arguments[0])))
    if len(entries) != expected_count:
        raise GameplayIdExtractionError(
            f"expected {expected_count} sound sources, extracted {len(entries)}"
        )
    _validate(entries, "sound source", "serialized_name")
    return entries
