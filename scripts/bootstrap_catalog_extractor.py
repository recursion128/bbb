"""Tree-sitter-backed extraction for vanilla bootstrap resource catalogs."""

from __future__ import annotations

import json
import math
import re
import struct
from collections import Counter
from dataclasses import dataclass

from vanilla_java_source import (
    JavaExpression,
    JavaField,
    JavaMethod,
    JavaSource,
    descendants,
    type_body_members,
)
from vanilla_tag_source import VanillaTagSourceError, resolve_tag_values


class BootstrapCatalogExtractionError(ValueError):
    """Explicit unsupported or inconsistent bootstrap source."""


@dataclass(frozen=True)
class BootstrapEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class RegistryCodecEntry:
    resource_id: str
    codec_owner: str


@dataclass(frozen=True)
class ConfiguredFeatureCodecEntry:
    resource_id: str
    feature_owner: str
    config_codec_owner: str


@dataclass(frozen=True)
class EnchantmentEffectComponentEntry:
    resource_id: str
    codec_family: str
    validation_context: str | None


@dataclass(frozen=True)
class LootContextParamEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class LootContextEntityTargetEntry:
    constant_name: str
    serialized_name: str
    context_param: str


@dataclass(frozen=True)
class LootContextParamSetEntry:
    constant_name: str
    resource_id: str
    required: tuple[str, ...]
    optional: tuple[str, ...]


@dataclass(frozen=True)
class WoodTypeEntry:
    constant_name: str
    name: str


@dataclass(frozen=True)
class WolfVariantEntry:
    constant_name: str
    resource_id: str
    file_name: str
    asset_ids: tuple[str, str, str, str, str, str]


@dataclass(frozen=True)
class CatVariantEntry:
    constant_name: str
    resource_id: str
    adult_asset_id: str
    baby_asset_id: str


@dataclass(frozen=True)
class FrogVariantEntry:
    constant_name: str
    resource_id: str
    asset_id: str


@dataclass(frozen=True)
class FarmAnimalVariantEntry:
    constant_name: str
    resource_id: str
    model_type: str
    adult_asset_id: str
    baby_asset_id: str


@dataclass(frozen=True)
class ModelAndTextureVariantEntry:
    constant_name: str
    resource_id: str
    model_type: str
    asset_id: str


@dataclass(frozen=True)
class BannerPatternEntry:
    constant_name: str
    resource_id: str
    asset_id: str
    translation_key: str


@dataclass(frozen=True)
class EnchantmentEntry:
    constant_name: str
    resource_id: str
    max_level: int


@dataclass(frozen=True)
class PaintingVariantEntry:
    constant_name: str
    resource_id: str
    width: int
    height: int
    has_author: bool


@dataclass(frozen=True)
class PotionEffectEntry:
    constant_name: str
    duration: int
    amplifier: int


@dataclass(frozen=True)
class PotionEntry:
    constant_name: str
    resource_id: str
    name: str
    effects: tuple[PotionEffectEntry, ...]


@dataclass(frozen=True)
class TrimMaterialEntry:
    constant_name: str
    resource_id: str
    description_color: int
    asset_name: str
    override_armor_assets: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class DecoratedPotPatternEntry:
    constant_name: str
    resource_id: str
    item_constant_name: str
    item_resource_id: str
    asset_id: str


@dataclass(frozen=True)
class MapDecorationEntry:
    constant_name: str
    resource_id: str
    asset_id: str
    show_on_item_frame: bool
    track_count: bool
    map_color: int | None = None
    exploration_map_element: bool = False


@dataclass(frozen=True)
class TimelineKeyframeEntry:
    ticks: int
    encoded_value: int


@dataclass(frozen=True)
class TimelineTrackEntry:
    attribute_name: str
    modifier: str
    value_kind: str
    keyframes: tuple[TimelineKeyframeEntry, ...]


@dataclass(frozen=True)
class OverworldDayTimelineEntry:
    period_ticks: int
    tracks: tuple[TimelineTrackEntry, ...]
    night_sky_light_color: int


def enchantment_tooltip_order(
    tag_resources: dict[str, tuple[str, bytes]],
    enchantments: list[EnchantmentEntry],
    expected_count: int = 43,
) -> tuple[str, ...]:
    try:
        entries = resolve_tag_values(
            tag_resources,
            "minecraft:tooltip_order",
            expected_count,
            registry_name="enchantment",
        )
    except VanillaTagSourceError as error:
        raise BootstrapCatalogExtractionError(str(error)) from error
    registered = {entry.resource_id for entry in enchantments}
    ordered = set(entries)
    if ordered != registered:
        raise BootstrapCatalogExtractionError(
            "data/minecraft/tags/enchantment/tooltip_order.json: "
            "tooltip order/enchantment registry mismatch: "
            f"unknown={sorted(ordered - registered)}, "
            f"missing={sorted(registered - ordered)}"
        )
    return entries


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise BootstrapCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected a string literal resource path")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "resource path is empty or non-string")
    return value


def _boolean(source: JavaSource, expression: JavaExpression) -> bool:
    if expression.canonical == "true":
        return True
    if expression.canonical == "false":
        return False
    _fail(source, expression, "expected a boolean literal")


def _invocation(
    source: JavaSource, expression: JavaExpression, expected_names: set[str]
) -> tuple[str, tuple[JavaExpression, ...]]:
    if expression.node.type != "method_invocation":
        _fail(source, expression, "expected a method invocation initializer")
    name_node = expression.node.child_by_field_name("name")
    arguments_node = expression.node.child_by_field_name("arguments")
    if name_node is None or arguments_node is None:
        _fail(source, expression, "method invocation lacks name or arguments")
    name = source.text(name_node)
    if name not in expected_names:
        _fail(source, expression, f"unsupported initializer call {name}")
    return name, tuple(
        source.expression(node) for node in arguments_node.named_children
    )


def _invocation_with_owner(
    source: JavaSource,
    expression: JavaExpression,
    expected_names: set[str],
) -> tuple[str | None, str, tuple[JavaExpression, ...]]:
    name, arguments = _invocation(source, expression, expected_names)
    object_node = expression.node.child_by_field_name("object")
    owner = source.expression(object_node).canonical if object_node is not None else None
    return owner, name, arguments


def _resource_key_fields(source: JavaSource, value_type: str) -> list[JavaField]:
    prefix = f"ResourceKey<{value_type}>"
    return [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith(prefix)
    ]


def _enum_string_argument(
    source: JavaSource, expression: JavaExpression
) -> str:
    reference = expression.canonical.replace(" ", "")
    parts = reference.split(".")
    if len(parts) < 2:
        _fail(source, expression, "resource key argument is neither string nor enum constant")
    enum_name, constant_name = parts[-2:]
    enum_types = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == enum_name
    ]
    if len(enum_types) != 1:
        _fail(source, expression, f"could not uniquely resolve enum {enum_name}")
    constants = [
        node
        for node in type_body_members(enum_types[0].body_node)
        if node.type == "enum_constant"
        and (name := node.child_by_field_name("name")) is not None
        and source.text(name) == constant_name
    ]
    if len(constants) != 1:
        _fail(source, expression, f"could not uniquely resolve enum constant {reference}")
    arguments = constants[0].child_by_field_name("arguments")
    if arguments is None or not arguments.named_children:
        _fail(source, expression, f"enum constant {reference} lacks constructor arguments")
    return _string(source, source.expression(arguments.named_children[0]))


def loot_context_param_entries(
    source: JavaSource, expected_count: int = 15
) -> list[LootContextParamEntry]:
    """Extract the declared ContextKey.vanilla identities in source order."""

    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith("ContextKey<")
    ]
    entries: list[LootContextParamEntry] = []
    for field in fields:
        if field.modifiers != ("public", "static", "final"):
            expression = (
                field.initializer
                if field.initializer is not None
                else JavaExpression("", field.span, field.node)
            )
            _fail(source, expression, f"loot context param {field.name} modifiers changed")
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"loot context param {field.name} lacks initializer",
            )
        owner, _, arguments = _invocation_with_owner(
            source, field.initializer, {"vanilla"}
        )
        if owner != "ContextKey" or len(arguments) != 1:
            _fail(
                source,
                field.initializer,
                f"loot context param {field.name} must call ContextKey.vanilla(name)",
            )
        entries.append(
            LootContextParamEntry(
                field.name,
                f"minecraft:{_string(source, arguments[0])}",
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} loot context params, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate loot context param constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate loot context param resource id"
        )
    return entries


def loot_context_entity_target_entries(
    source: JavaSource,
    params: list[LootContextParamEntry],
    expected_count: int = 6,
) -> list[LootContextEntityTargetEntry]:
    """Extract EntityTarget serialized names and ContextKey mappings in enum order."""

    entity_target_types = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == "EntityTarget"
    ]
    if len(entity_target_types) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one EntityTarget enum"
        )
    entries: list[LootContextEntityTargetEntry] = []
    known_params = {entry.constant_name for entry in params}
    for node in type_body_members(entity_target_types[0].body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None or len(arguments.named_children) != 2:
            _fail(
                source,
                source.expression(node),
                "EntityTarget constant must have name and context-param arguments",
            )
        serialized_name = _string(
            source, source.expression(arguments.named_children[0])
        )
        context_param_expression = source.expression(arguments.named_children[1])
        context_param_parts = context_param_expression.canonical.replace(" ", "").split(".")
        if len(context_param_parts) != 2 or context_param_parts[0] != "LootContextParams":
            _fail(
                source,
                context_param_expression,
                "EntityTarget context param must reference LootContextParams",
            )
        context_param = context_param_parts[1]
        if context_param not in known_params:
            _fail(
                source,
                context_param_expression,
                f"EntityTarget references unknown loot context param {context_param}",
            )
        entries.append(
            LootContextEntityTargetEntry(
                source.text(name), serialized_name, context_param
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} loot entity targets, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate loot entity target constant"
        )
    if len({entry.serialized_name for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate loot entity target serialized name"
        )
    if len({entry.context_param for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate loot entity target context param"
        )
    return entries


def _validate_loot_context_param_set_register_helper(source: JavaSource) -> None:
    helpers = [method for method in source.methods if method.name == "register"]
    if len(helpers) != 1 or helpers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one loot context param-set "
            "register helper with a body"
        )
    helper = helpers[0]
    if tuple(parameter.canonical for parameter in helper.parameters) != (
        "final String name",
        "final Consumer <ContextKeySet.Builder> consumer",
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: loot context param-set register helper parameters changed"
        )
    body = helper.node.child_by_field_name("body")
    if body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: loot context param-set register helper body changed"
        )
    statements = tuple(
        (statement.type, source.expression(statement).canonical)
        for statement in body.named_children
    )
    expected = (
        (
            "local_variable_declaration",
            "ContextKeySet.Builder builder = new ContextKeySet.Builder ();",
        ),
        ("expression_statement", "consumer.accept (builder);"),
        ("local_variable_declaration", "ContextKeySet result = builder.build ();"),
        (
            "local_variable_declaration",
            "Identifier id = Identifier.withDefaultNamespace (name);",
        ),
        (
            "local_variable_declaration",
            "ContextKeySet prev = REGISTRY.put (id, result);",
        ),
        (
            "if_statement",
            'if (prev != null) {throw new IllegalStateException ("Loot table parameter set " + id + " is already registered");} else {return result;}',
        ),
    )
    if statements != expected:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: loot context param-set register helper/REGISTRY "
            "sentinel shape changed"
        )


def _loot_context_param_set_builder_entries(
    source: JavaSource,
    expression: JavaExpression,
    parameter_name: str,
    known_params: set[str],
) -> list[tuple[str, str]]:
    if expression.node.type != "method_invocation":
        _fail(source, expression, "loot context param-set builder body must be a call chain")
    name_node = expression.node.child_by_field_name("name")
    arguments_node = expression.node.child_by_field_name("arguments")
    object_node = expression.node.child_by_field_name("object")
    if name_node is None or arguments_node is None or object_node is None:
        _fail(source, expression, "loot context param-set builder call shape changed")
    operation = source.text(name_node)
    if operation not in {"required", "optional"}:
        _fail(source, expression, f"unsupported loot context param-set builder call {operation}")
    if len(arguments_node.named_children) != 1:
        _fail(source, expression, f"loot context param-set {operation} arity changed")
    param_expression = source.expression(arguments_node.named_children[0])
    param_parts = param_expression.canonical.replace(" ", "").split(".")
    if len(param_parts) != 2 or param_parts[0] != "LootContextParams":
        _fail(
            source,
            param_expression,
            "loot context param-set builder argument must reference LootContextParams",
        )
    param = param_parts[1]
    if param not in known_params:
        _fail(
            source,
            param_expression,
            f"loot context param-set references unknown loot context param {param}",
        )
    object_expression = source.expression(object_node)
    if object_node.type == "identifier":
        if object_expression.canonical != parameter_name:
            _fail(source, object_expression, "loot context param-set builder owner changed")
        prefix: list[tuple[str, str]] = []
    else:
        prefix = _loot_context_param_set_builder_entries(
            source, object_expression, parameter_name, known_params
        )
    prefix.append((operation, param))
    return prefix


def loot_context_param_set_entries(
    source: JavaSource,
    params: list[LootContextParamEntry],
    expected_count: int = 26,
) -> list[LootContextParamSetEntry]:
    """Extract ContextKeySet registrations and builder membership in source order."""

    _validate_loot_context_param_set_register_helper(source)
    known_params = {entry.constant_name for entry in params}
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "") == "ContextKeySet"
    ]
    entries: list[LootContextParamSetEntry] = []
    for field in fields:
        if field.modifiers != ("public", "static", "final"):
            expression = field.initializer or JavaExpression("", field.span, field.node)
            _fail(source, expression, f"loot context param set {field.name} modifiers changed")
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"loot context param set {field.name} lacks initializer",
            )
        owner, _, arguments = _invocation_with_owner(
            source, field.initializer, {"register"}
        )
        if owner is not None or len(arguments) != 2:
            _fail(
                source,
                field.initializer,
                "loot context param set must call register(id, builder)",
            )
        resource_path, builder = arguments
        if builder.node.type != "lambda_expression":
            _fail(source, builder, "loot context param-set builder must be a lambda")
        parameters = builder.node.child_by_field_name("parameters")
        body = builder.node.child_by_field_name("body")
        if parameters is None or parameters.type != "identifier" or body is None:
            _fail(source, builder, "loot context param-set builder lambda shape changed")
        parameter_name = source.text(parameters)
        if body.type == "block":
            if body.named_children:
                _fail(source, source.expression(body), "non-empty block builder is unsupported")
            memberships: list[tuple[str, str]] = []
        else:
            memberships = _loot_context_param_set_builder_entries(
                source, source.expression(body), parameter_name, known_params
            )
        member_names = [param for _, param in memberships]
        if len(set(member_names)) != len(member_names):
            _fail(source, builder, f"loot context param set {field.name} repeats a param")
        entries.append(
            LootContextParamSetEntry(
                field.name,
                f"minecraft:{_string(source, resource_path)}",
                tuple(param for operation, param in memberships if operation == "required"),
                tuple(param for operation, param in memberships if operation == "optional"),
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} loot context param sets, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate loot context param-set constant"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate loot context param-set resource id"
        )
    return entries


def _declared_resource_keys(
    source: JavaSource,
    value_type: str,
    initializer_names: set[str],
    referenced_paths: dict[str, str] | None = None,
) -> dict[str, str]:
    resources: dict[str, str] = {}
    for field in _resource_key_fields(source, value_type):
        if field.initializer is None or field.initializer.node.type != "method_invocation":
            continue
        name_node = field.initializer.node.child_by_field_name("name")
        if name_node is None or source.text(name_node) not in initializer_names:
            continue
        _, arguments = _invocation(source, field.initializer, initializer_names)
        if len(arguments) != 1:
            _fail(source, field.initializer, f"resource key {field.name} must have one argument")
        if arguments[0].node.type == "string_literal":
            path = _string(source, arguments[0])
        elif referenced_paths is not None and arguments[0].canonical in referenced_paths:
            path = referenced_paths[arguments[0].canonical]
        else:
            path = _enum_string_argument(source, arguments[0])
        if field.name in resources:
            _fail(source, field.initializer, f"duplicate resource key {field.name}")
        resources[field.name] = f"minecraft:{path}"
    if not resources:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: no ResourceKey<{value_type}> declarations found"
        )
    return resources


def _bootstrap_method(source: JavaSource) -> JavaMethod:
    methods = [method for method in source.methods if method.name == "bootstrap"]
    if len(methods) != 1 or methods[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one bootstrap method with a body"
        )
    return methods[0]


def _bootstrap_context_name(source: JavaSource, method: JavaMethod) -> str:
    parameters = method.node.child_by_field_name("parameters")
    if parameters is None or len(parameters.named_children) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: bootstrap method must have exactly one parameter"
        )
    name = parameters.named_children[0].child_by_field_name("name")
    if name is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: bootstrap parameter lacks a name"
        )
    return source.text(name)


def bootstrap_resource_key_entries(
    source: JavaSource,
    value_type: str,
    initializer_names: set[str],
    expected_count: int,
    *,
    referenced_paths: dict[str, str] | None = None,
    registration_names: set[str] | None = None,
) -> list[BootstrapEntry]:
    resources = _declared_resource_keys(
        source, value_type, initializer_names, referenced_paths
    )
    method = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, method)
    registration_names = registration_names or {"register"}
    calls = [
        call
        for call in source.calls
        if call.name in registration_names
        and (
            call.qualifier is None
            or call.qualifier.canonical == context_name
        )
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    registered: list[str] = []
    for call in calls:
        if call.qualifier is None:
            if len(call.arguments) < 2 or call.arguments[0].canonical != context_name:
                _fail(source, source.expression(call.node), "unsupported bootstrap register call")
            key = call.arguments[1]
        else:
            if len(call.arguments) < 2:
                _fail(source, source.expression(call.node), "unsupported bootstrap register call")
            key = call.arguments[0]
        if key.node.type != "identifier":
            _fail(source, key, "bootstrap resource key is not an identifier")
        registered.append(key.canonical)
    if len(registered) != len(set(registered)):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate bootstrap registration"
        )
    unknown = set(registered) - resources.keys()
    missing = resources.keys() - set(registered)
    if unknown or missing:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: declaration/bootstrap mismatch: "
            f"unknown={sorted(unknown)}, missing={sorted(missing)}"
        )
    if len(registered) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} {value_type} entries, "
            f"extracted {len(registered)}"
        )
    entries = [BootstrapEntry(name, resources[name]) for name in registered]
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate {value_type} resource id"
        )
    return entries


def _qualified_identifier_parts(
    source: JavaSource, expression: JavaExpression
) -> tuple[str, ...]:
    def parts(node) -> tuple[str, ...]:
        if node.type == "identifier":
            return (source.text(node),)
        if node.type != "field_access":
            _fail(source, source.expression(node), "codec is not a qualified field reference")
        object_node = node.child_by_field_name("object")
        field_node = node.child_by_field_name("field")
        if object_node is None or field_node is None or field_node.type != "identifier":
            _fail(
                source,
                source.expression(node),
                "codec field reference has unsupported structure",
            )
        return (*parts(object_node), source.text(field_node))

    return parts(expression.node)


def registry_codec_entries(
    source: JavaSource,
    expected_count: int,
    *,
    codec_terminals: frozenset[str] = frozenset({"CODEC"}),
) -> list[RegistryCodecEntry]:
    """Extracts ordered `Registry.register(registry, id, Owner.CODEC)` calls."""

    supported_terminals = frozenset({"CODEC", "MAP_CODEC"})
    if not codec_terminals or not codec_terminals <= supported_terminals:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: codec terminals must be a non-empty subset of "
            f"{sorted(supported_terminals)}, got {sorted(codec_terminals)}"
        )

    method = _bootstrap_method(source)
    registry_name = _bootstrap_context_name(source, method)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[RegistryCodecEntry] = []
    for call in calls:
        call_expression = source.expression(call.node)
        if call.qualifier is None or call.qualifier.canonical != "Registry":
            _fail(source, call_expression, "unsupported bootstrap register owner")
        if len(call.arguments) != 3:
            _fail(source, call_expression, "Registry.register must have exactly three arguments")
        registry, resource_path, codec = call.arguments
        if registry.node.type != "identifier" or registry.canonical != registry_name:
            _fail(source, registry, "Registry.register does not target the bootstrap registry")
        codec_parts = _qualified_identifier_parts(source, codec)
        if len(codec_parts) < 2 or codec_parts[-1] not in codec_terminals:
            _fail(
                source,
                codec,
                "registered codec terminal must be one of "
                f"{sorted(codec_terminals)}",
            )
        entries.append(
            RegistryCodecEntry(
                f"minecraft:{_string(source, resource_path)}",
                ".".join(codec_parts[:-1]),
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} registry codec entries, "
            f"extracted {len(entries)}"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate registry codec resource id"
        )
    if len({entry.codec_owner for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate registry codec owner"
        )
    return entries


def _validate_registry_helper(
    source: JavaSource,
    registry_owner: str,
    expected_payload: str | None = None,
) -> None:
    helpers = [method for method in source.methods if method.name == "register"]
    if len(helpers) != 1 or helpers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one register helper with a body"
        )
    helper = helpers[0]
    registrations = [
        call
        for call in source.calls
        if call.name == "register"
        and helper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= helper.body.end_byte
    ]
    if len(registrations) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: register helper must contain one Registry.register call"
        )
    registration = registrations[0]
    expression = source.expression(registration.node)
    if registration.qualifier is None or registration.qualifier.canonical != "Registry":
        _fail(source, expression, "register helper does not call Registry.register")
    if len(registration.arguments) != 3:
        _fail(source, expression, "register helper Registry.register arity changed")
    registry, resource_id, payload = registration.arguments
    if registry.canonical != registry_owner:
        _fail(source, registry, f"register helper target changed from {registry_owner}")
    if resource_id.canonical not in {"id", "name"}:
        _fail(source, resource_id, "register helper resource id forwarding changed")
    if expected_payload is not None and payload.canonical.replace(" ", "") != expected_payload:
        _fail(source, payload, "register helper payload construction changed")


def static_registry_codec_entries(
    source: JavaSource,
    field_type: str,
    registry_owner: str,
    registry_payload: str,
    expected_count: int,
    *,
    codec_terminals: frozenset[str] = frozenset({"CODEC"}),
    bootstrap_sentinel: str | None = None,
) -> list[RegistryCodecEntry]:
    """Extract ordered codec registrations stored in static fields."""

    supported_terminals = frozenset({"CODEC", "MAP_CODEC"})
    if not codec_terminals or not codec_terminals <= supported_terminals:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: codec terminals must be a non-empty subset of "
            f"{sorted(supported_terminals)}, got {sorted(codec_terminals)}"
        )
    _validate_registry_helper(source, registry_owner, registry_payload)
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith(field_type)
    ]
    entries: list[RegistryCodecEntry] = []
    for field in fields:
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"{field.name} lacks initializer",
            )
        owner, _, arguments = _invocation_with_owner(
            source, field.initializer, {"register"}
        )
        if owner is not None or len(arguments) != 2:
            _fail(source, field.initializer, "static codec field must call register(id, codec)")
        resource_path, codec = arguments
        codec_parts = _qualified_identifier_parts(source, codec)
        if len(codec_parts) < 2 or codec_parts[-1] not in codec_terminals:
            _fail(
                source,
                codec,
                "registered codec terminal must be one of "
                f"{sorted(codec_terminals)}",
            )
        entries.append(
            RegistryCodecEntry(
                f"minecraft:{_string(source, resource_path)}",
                ".".join(codec_parts[:-1]),
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} static registry codec entries, "
            f"extracted {len(entries)}"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate static registry codec resource id"
        )
    if len({entry.codec_owner for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate static registry codec owner"
        )
    if bootstrap_sentinel is not None:
        bootstraps = [method for method in source.methods if method.name == "bootstrap"]
        if len(bootstraps) != 1 or bootstraps[0].body is None:
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: expected one bootstrap sentinel method"
            )
        returns = [
            node
            for node in descendants(bootstraps[0].node)
            if node.type == "return_statement"
        ]
        if len(returns) != 1 or len(returns[0].named_children) != 1:
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: bootstrap sentinel return shape changed"
            )
        returned = source.expression(returns[0].named_children[0])
        if returned.canonical != bootstrap_sentinel:
            _fail(source, returned, f"bootstrap sentinel changed from {bootstrap_sentinel}")
    return entries


def placement_modifier_codec_entries(
    source: JavaSource, expected_count: int = 15
) -> list[RegistryCodecEntry]:
    """Extract the PlacementModifierType dispatch inventory and MapCodec contract."""

    codec_methods = [method for method in source.methods if method.name == "codec"]
    if len(codec_methods) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one placement-modifier codec method"
        )
    codec_method = codec_methods[0]
    codec_return = codec_method.node.child_by_field_name("type")
    if (
        codec_method.body is not None
        or codec_method.parameters
        or codec_return is None
        or source.text(codec_return).replace(" ", "") != "MapCodec<P>"
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: placement-modifier MapCodec owner method changed"
        )

    helpers = [method for method in source.methods if method.name == "register"]
    if len(helpers) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one placement-modifier register helper"
        )
    helper = helpers[0]
    helper_return = helper.node.child_by_field_name("type")
    helper_parts = {
        child.type: source.text(child).replace(" ", "")
        for child in helper.node.named_children
    }
    if (
        helper_parts.get("modifiers") != "privatestatic"
        or helper_parts.get("type_parameters") != "<PextendsPlacementModifier>"
        or helper_return is None
        or source.text(helper_return).replace(" ", "") != "PlacementModifierType<P>"
        or tuple(parameter.canonical.replace(" ", "") for parameter in helper.parameters)
        != ("finalStringid", "finalMapCodec<P>codec")
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: placement-modifier register helper signature changed"
        )

    return static_registry_codec_entries(
        source,
        "PlacementModifierType<",
        "BuiltInRegistries.PLACEMENT_MODIFIER_TYPE",
        "()->codec",
        expected_count,
        codec_terminals=frozenset({"CODEC"}),
    )


def configured_feature_codec_entries(
    source: JavaSource, expected_count: int = 60
) -> list[ConfiguredFeatureCodecEntry]:
    """Extract ``Feature`` registration and configuration-codec identities.

    The concrete constructed feature is authoritative.  In particular, the
    declaration's left-hand type is deliberately not used because Vanilla's
    ``SEAGRASS`` and ``BONUS_CHEST`` declarations use their concrete types.
    """

    feature_types = [type_ for type_ in source.types if type_.simple_name == "Feature"]
    if len(feature_types) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one Feature type"
        )
    feature_type = feature_types[0]
    type_parameters = feature_type.node.child_by_field_name("type_parameters")
    if (
        feature_type.kind != "class"
        or type_parameters is None
        or source.text(type_parameters).replace(" ", "")
        != "<FCextendsFeatureConfiguration>"
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: Feature type contract changed"
        )

    helpers = [
        method
        for method in source.methods_for(feature_type)
        if method.name == "register" and not method.is_constructor
    ]
    if len(helpers) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one Feature register helper"
        )
    helper = helpers[0]
    helper_return = helper.node.child_by_field_name("type")
    helper_parts = {
        child.type: source.text(child).replace(" ", "")
        for child in helper.node.named_children
    }
    if (
        helper.body is None
        or helper_parts.get("modifiers") != "privatestatic"
        or helper_parts.get("type_parameters")
        != "<CextendsFeatureConfiguration,FextendsFeature<C>>"
        or helper_return is None
        or source.text(helper_return).replace(" ", "") != "F"
        or tuple(parameter.canonical.replace(" ", "") for parameter in helper.parameters)
        != ("finalStringname", "finalFfeature")
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: Feature register helper signature changed"
        )
    _validate_registry_helper(source, "BuiltInRegistries.FEATURE", "feature")

    configured_fields = [
        field
        for field in source.fields_for(feature_type)
        if field.name == "configuredCodec"
    ]
    if (
        len(configured_fields) != 1
        or configured_fields[0].modifiers != ("private", "final")
        or configured_fields[0].type_spelling.replace(" ", "")
        != "MapCodec<ConfiguredFeature<FC,Feature<FC>>>"
        or configured_fields[0].initializer is not None
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: Feature configuredCodec field changed"
        )

    constructors = [
        method for method in source.methods_for(feature_type) if method.is_constructor
    ]
    if len(constructors) != 1 or constructors[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one Feature constructor"
        )
    constructor = constructors[0]
    constructor_modifiers = next(
        (
            source.text(child).replace(" ", "")
            for child in constructor.node.named_children
            if child.type == "modifiers"
        ),
        "",
    )
    assignments = [
        node
        for node in descendants(constructor.node)
        if node.type == "assignment_expression"
    ]
    expected_assignment = (
        'this.configuredCodec=codec.fieldOf("config").xmap('
        "c->newConfiguredFeature<>(this,(FC)c),ConfiguredFeature::config)"
    )
    if (
        constructor_modifiers != "public"
        or tuple(
            parameter.canonical.replace(" ", "")
            for parameter in constructor.parameters
        )
        != ("finalCodec<FC>codec",)
        or len(assignments) != 1
        or source.expression(assignments[0]).canonical.replace(" ", "")
        != expected_assignment
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: Feature configured codec construction changed"
        )

    accessors = [
        method
        for method in source.methods_for(feature_type)
        if method.name == "configuredCodec" and not method.is_constructor
    ]
    if len(accessors) != 1 or accessors[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one configuredCodec accessor"
        )
    accessor = accessors[0]
    accessor_return = accessor.node.child_by_field_name("type")
    accessor_modifiers = next(
        (
            source.text(child).replace(" ", "")
            for child in accessor.node.named_children
            if child.type == "modifiers"
        ),
        "",
    )
    returns = [
        node for node in descendants(accessor.node) if node.type == "return_statement"
    ]
    if (
        accessor_modifiers != "public"
        or accessor_return is None
        or source.text(accessor_return).replace(" ", "")
        != "MapCodec<ConfiguredFeature<FC,Feature<FC>>>"
        or accessor.parameters
        or len(returns) != 1
        or source.expression(returns[0]).canonical.replace(" ", "")
        != "returnthis.configuredCodec;"
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: Feature configuredCodec accessor changed"
        )

    entries: list[ConfiguredFeatureCodecEntry] = []
    for field in source.fields_for(feature_type):
        if field.initializer is None or field.initializer.node.type != "method_invocation":
            continue
        name_node = field.initializer.node.child_by_field_name("name")
        if name_node is None or source.text(name_node) != "register":
            continue
        if field.modifiers != ("public", "static", "final"):
            _fail(source, field.initializer, "Feature registration modifiers changed")
        owner, _, arguments = _invocation_with_owner(
            source, field.initializer, {"register"}
        )
        if owner is not None or len(arguments) != 2:
            _fail(source, field.initializer, "Feature field must call register(id, feature)")
        resource_path, feature = arguments
        if feature.node.type != "object_creation_expression":
            _fail(source, feature, "registered Feature is not constructed inline")
        feature_owner_node = feature.node.child_by_field_name("type")
        constructor_arguments = feature.node.child_by_field_name("arguments")
        if feature_owner_node is None or constructor_arguments is None:
            _fail(source, feature, "registered Feature constructor is incomplete")
        if len(constructor_arguments.named_children) != 1:
            _fail(source, feature, "registered Feature must receive one configuration codec")
        feature_owner = source.expression(feature_owner_node).canonical.replace(" ", "")
        if not feature_owner:
            _fail(source, feature, "registered Feature owner is empty")
        codec = source.expression(constructor_arguments.named_children[0])
        codec_parts = _qualified_identifier_parts(source, codec)
        if len(codec_parts) < 2 or codec_parts[-1] != "CODEC":
            _fail(source, codec, "Feature configuration codec must end in CODEC")
        entries.append(
            ConfiguredFeatureCodecEntry(
                f"minecraft:{_string(source, resource_path)}",
                feature_owner,
                ".".join(codec_parts[:-1]),
            )
        )

    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} Feature registrations, "
            f"extracted {len(entries)}"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate configured Feature resource id"
        )
    if len({entry.feature_owner for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate concrete Feature owner"
        )
    if expected_count == 60:
        family_counts = Counter(entry.config_codec_owner for entry in entries)
        repeated = {owner: count for owner, count in family_counts.items() if count > 1}
        expected_repeated = {
            "NoneFeatureConfiguration": 18,
            "HugeMushroomFeatureConfiguration": 2,
            "OreConfiguration": 2,
            "ProbabilityFeatureConfiguration": 2,
            "VegetationPatchConfiguration": 2,
        }
        if (
            len(family_counts) != 39
            or repeated != expected_repeated
            or entries[0].resource_id != "minecraft:no_op"
            or entries[-1].resource_id != "minecraft:sculk_patch"
        ):
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: configured Feature family/order contract changed: "
                f"families={len(family_counts)}, repeated={repeated}, "
                f"first={entries[0].resource_id}, last={entries[-1].resource_id}"
            )
    return entries


def world_carver_codec_entries(
    source: JavaSource, expected_count: int = 3
) -> list[RegistryCodecEntry]:
    """Extract the configured-carver dispatch families from ``WorldCarver``.

    Unlike the ordinary static codec registries, the registered value is the
    carver instance and its configured codec is constructed by the superclass.
    The field/resource identities are nevertheless the authoritative dispatch
    inventory, so retain those while also pinning the constructor's CODEC
    argument shape.
    """

    _validate_registry_helper(source, "BuiltInRegistries.CARVER", "carver")
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith("WorldCarver<")
    ]
    entries: list[RegistryCodecEntry] = []
    for field in fields:
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"{field.name} lacks initializer",
            )
        owner, _, arguments = _invocation_with_owner(
            source, field.initializer, {"register"}
        )
        if owner is not None or len(arguments) != 2:
            _fail(source, field.initializer, "carver field must call register(id, carver)")
        resource_path, carver = arguments
        if carver.node.type != "object_creation_expression":
            _fail(source, carver, "registered carver is not constructed inline")
        constructor_arguments = carver.node.child_by_field_name("arguments")
        if constructor_arguments is None or len(constructor_arguments.named_children) != 1:
            _fail(source, carver, "carver constructor must receive one configuration codec")
        codec = source.expression(constructor_arguments.named_children[0])
        codec_parts = _qualified_identifier_parts(source, codec)
        if len(codec_parts) < 2 or codec_parts[-1] != "CODEC":
            _fail(source, codec, "carver configuration must end in CODEC")
        family = "".join(part.title() for part in field.name.lower().split("_"))
        entries.append(
            RegistryCodecEntry(
                f"minecraft:{_string(source, resource_path)}",
                family,
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} carvers, extracted {len(entries)}"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate carver resource id"
        )
    return entries


_VALIDATED_EFFECT_CODEC_FAMILIES = {
    ("ConditionalEffect", "codec", "EnchantmentValueEffect"): "ConditionalValueEffect",
    ("ConditionalEffect", "codec", "DamageImmunity"): "ConditionalDamageImmunity",
    ("ConditionalEffect", "codec", "EnchantmentEntityEffect"): "ConditionalEntityEffect",
    (
        "ConditionalEffect",
        "codec",
        "EnchantmentLocationBasedEffect",
    ): "ConditionalLocationBasedEffect",
    (
        "TargetedConditionalEffect",
        "codec",
        "EnchantmentEntityEffect",
    ): "TargetedEntityEffect",
    (
        "TargetedConditionalEffect",
        "equipmentDropsCodec",
        "EnchantmentValueEffect",
    ): "EquipmentDropsValueEffect",
}

_DIRECT_EFFECT_CODEC_FAMILIES = {
    "EnchantmentAttributeEffect.MAP_CODEC.codec().listOf()": "AttributeEffects",
    "EnchantmentValueEffect.CODEC": "ValueEffect",
    "CrossbowItem.ChargingSounds.CODEC.listOf()": "CrossbowChargingSounds",
    "SoundEvent.CODEC.listOf()": "SoundEvents",
    "Unit.CODEC": "Unit",
}


def _effect_component_codec_fact(
    source: JavaSource,
    codec: JavaExpression,
) -> tuple[str, str | None]:
    normalized = codec.canonical.replace(" ", "")
    direct_family = _DIRECT_EFFECT_CODEC_FAMILIES.get(normalized)
    if direct_family is not None:
        return direct_family, None
    owner, _, arguments = _invocation_with_owner(
        source, codec, {"validatedListCodec"}
    )
    if owner is not None or len(arguments) != 2:
        _fail(source, codec, "unsupported effect-component codec expression")
    wrapped_codec, context = arguments
    wrapper_owner, wrapper_name, wrapper_arguments = _invocation_with_owner(
        source,
        wrapped_codec,
        {"codec", "equipmentDropsCodec"},
    )
    if wrapper_owner is None or len(wrapper_arguments) != 1:
        _fail(source, wrapped_codec, "unsupported validated effect codec wrapper")
    codec_parts = _qualified_identifier_parts(source, wrapper_arguments[0])
    if len(codec_parts) < 2 or codec_parts[-1] != "CODEC":
        _fail(source, wrapper_arguments[0], "validated effect codec must end in CODEC")
    key = (wrapper_owner.replace(" ", ""), wrapper_name, ".".join(codec_parts[:-1]))
    family = _VALIDATED_EFFECT_CODEC_FAMILIES.get(key)
    if family is None:
        _fail(source, wrapped_codec, f"unsupported validated effect codec family {key}")
    context_parts = _qualified_identifier_parts(source, context)
    if len(context_parts) != 2 or context_parts[0] != "LootContextParamSets":
        _fail(source, context, "unsupported effect validation context")
    return family, context_parts[1]


def enchantment_effect_component_entries(
    source: JavaSource,
    expected_count: int,
) -> list[EnchantmentEffectComponentEntry]:
    """Extract effect-component registration order, codec family, and validation context."""

    _validate_registry_helper(
        source,
        "BuiltInRegistries.ENCHANTMENT_EFFECT_COMPONENT_TYPE",
        "builder.apply(DataComponentType.builder()).build()",
    )
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith("DataComponentType<")
    ]
    entries: list[EnchantmentEffectComponentEntry] = []
    for field in fields:
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"{field.name} lacks initializer",
            )
        owner, _, arguments = _invocation_with_owner(
            source, field.initializer, {"register"}
        )
        if owner is not None or len(arguments) != 2:
            _fail(source, field.initializer, "effect component must call register(id, builder)")
        resource_path, builder = arguments
        if builder.node.type != "lambda_expression":
            _fail(source, builder, "effect component builder must be a lambda")
        parameters = builder.node.child_by_field_name("parameters")
        body = builder.node.child_by_field_name("body")
        if parameters is None or body is None or parameters.type != "identifier":
            _fail(source, builder, "effect component builder lambda shape changed")
        parameter_name = source.text(parameters)
        body_expression = source.expression(body)
        persistent_owner, _, persistent_arguments = _invocation_with_owner(
            source, body_expression, {"persistent"}
        )
        if persistent_owner != parameter_name or len(persistent_arguments) != 1:
            _fail(
                source,
                body_expression,
                "effect component builder must call parameter.persistent(codec)",
            )
        codec_family, validation_context = _effect_component_codec_fact(
            source, persistent_arguments[0]
        )
        entries.append(
            EnchantmentEffectComponentEntry(
                f"minecraft:{_string(source, resource_path)}",
                codec_family,
                validation_context,
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} enchantment effect components, "
            f"extracted {len(entries)}"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate enchantment effect component resource id"
        )
    bootstraps = [method for method in source.methods if method.name == "bootstrap"]
    if len(bootstraps) != 1 or bootstraps[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one effect-component bootstrap method"
        )
    returns = [
        node
        for node in descendants(bootstraps[0].node)
        if node.type == "return_statement"
    ]
    if len(returns) != 1 or len(returns[0].named_children) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: effect-component bootstrap return shape changed"
        )
    returned = source.expression(returns[0].named_children[0])
    if returned.canonical != "DAMAGE_PROTECTION":
        _fail(source, returned, "effect-component bootstrap sentinel changed")
    return entries


def sound_variant_entries(
    source: JavaSource,
    value_type: str,
    sound_set_owner: str,
    sound_map_field: str,
    expected_count: int,
) -> list[BootstrapEntry]:
    entries = bootstrap_resource_key_entries(
        source,
        value_type,
        {"createKey"},
        expected_count,
    )
    bootstrap = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, bootstrap)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and bootstrap.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= bootstrap.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    if len(calls) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: sound-variant bootstrap registration count changed"
        )
    for call, entry in zip(calls, entries, strict=True):
        expected_sound_set = (
            f"{sound_set_owner}.SoundSet.{entry.constant_name}"
        )
        if (
            len(call.arguments) != 3
            or call.arguments[0].canonical != context_name
            or call.arguments[1].canonical != entry.constant_name
            or call.arguments[2].canonical != expected_sound_set
        ):
            _fail(
                source,
                source.expression(call.node),
                "sound-variant bootstrap payload changed",
            )

    helpers = [
        method
        for method in source.methods
        if method.name == "register"
        and len(method.parameters) == 3
        and method.body is not None
    ]
    if len(helpers) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one sound-variant register helper"
        )
    helper = helpers[0]
    parameter_names = []
    for parameter in helper.parameters:
        name = parameter.node.child_by_field_name("name")
        if name is None:
            _fail(source, parameter, "sound-variant helper parameter lacks a name")
        parameter_names.append(source.text(name))
    helper_calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == parameter_names[0]
        and helper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= helper.body.end_byte
    ]
    expected_helper = (
        f"{parameter_names[0]}.register ({parameter_names[1]}, "
        f"SoundEvents.{sound_map_field}.get ({parameter_names[2]}))"
    )
    if (
        len(helper_calls) != 1
        or source.expression(helper_calls[0].node).canonical != expected_helper
    ):
        expression = (
            source.expression(helper_calls[0].node)
            if helper_calls
            else source.expression(helper.node)
        )
        _fail(source, expression, "sound-variant register helper payload changed")
    return entries


def wolf_variant_entries(
    source: JavaSource, expected_count: int = 9
) -> list[WolfVariantEntry]:
    identities = bootstrap_resource_key_entries(
        source,
        "WolfVariant",
        {"createKey"},
        expected_count,
    )
    method = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, method)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body is not None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    file_names: dict[str, str] = {}
    for call in calls:
        if len(call.arguments) != 4 or call.arguments[0].canonical != context_name:
            _fail(
                source,
                source.expression(call.node),
                "wolf bootstrap register call must have context, key, file name, and selectors",
            )
        key = call.arguments[1]
        if key.node.type != "identifier":
            _fail(source, key, "wolf bootstrap resource key is not an identifier")
        file_names[key.canonical] = _string(source, call.arguments[2])

    helpers = []
    for candidate in source.methods:
        if candidate.name != "register" or len(candidate.parameters) != 4:
            continue
        last_type = candidate.parameters[3].node.child_by_field_name("type")
        if (
            last_type is not None
            and source.expression(last_type).canonical.replace(" ", "")
            == "SpawnPrioritySelectors"
        ):
            helpers.append(candidate)
    if len(helpers) != 1 or helpers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one four-parameter wolf asset register helper"
        )
    helper = helpers[0]
    parameter_names = []
    for parameter in helper.parameters:
        name = parameter.node.child_by_field_name("name")
        if name is None:
            _fail(source, parameter, "wolf asset helper parameter lacks a name")
        parameter_names.append(source.text(name))
    if parameter_names != ["context", "name", "fileName", "selectors"]:
        _fail(source, helper.parameters[0], "wolf asset helper parameters changed")

    expected_initializers = {
        "wildTexture": 'Identifier.withDefaultNamespace ("entity/wolf/" + fileName)',
        "tameTexture": 'Identifier.withDefaultNamespace ("entity/wolf/" + fileName + "_tame")',
        "angryTexture": 'Identifier.withDefaultNamespace ("entity/wolf/" + fileName + "_angry")',
        "babyTexture": 'Identifier.withDefaultNamespace ("entity/wolf/" + fileName + "_baby")',
        "tameBabyTexture": 'Identifier.withDefaultNamespace ("entity/wolf/" + fileName + "_tame_baby")',
        "angryBabyTexture": 'Identifier.withDefaultNamespace ("entity/wolf/" + fileName + "_angry_baby")',
    }
    initializers: dict[str, JavaExpression] = {}
    for node in descendants(helper.node):
        if node.type != "variable_declarator":
            continue
        name = node.child_by_field_name("name")
        value = node.child_by_field_name("value")
        if name is None or value is None:
            _fail(source, source.expression(node), "wolf asset local lacks name or initializer")
        initializers[source.text(name)] = source.expression(value)
    if set(initializers) != set(expected_initializers):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: wolf asset helper locals changed: "
            f"expected={sorted(expected_initializers)}, actual={sorted(initializers)}"
        )
    for name, expected in expected_initializers.items():
        if initializers[name].canonical != expected:
            _fail(source, initializers[name], f"wolf asset formula for {name} changed")

    registrations = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "context"
        and helper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= helper.body.end_byte
    ]
    expected_registration = (
        "context.register (name, new WolfVariant (new WolfVariant.AssetInfo "
        "(new ClientAsset.ResourceTexture (wildTexture), new ClientAsset.ResourceTexture "
        "(tameTexture), new ClientAsset.ResourceTexture (angryTexture)), new WolfVariant.AssetInfo "
        "(new ClientAsset.ResourceTexture (babyTexture), new ClientAsset.ResourceTexture "
        "(tameBabyTexture), new ClientAsset.ResourceTexture (angryBabyTexture)), selectors))"
    )
    if (
        len(registrations) != 1
        or source.expression(registrations[0].node).canonical != expected_registration
    ):
        expression = (
            source.expression(registrations[0].node)
            if registrations
            else source.expression(helper.node)
        )
        _fail(source, expression, "wolf asset registration payload changed")

    entries = []
    for identity in identities:
        file_name = file_names.get(identity.constant_name)
        if file_name is None:
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: wolf variant {identity.constant_name} lacks a file name"
            )
        prefix = f"minecraft:entity/wolf/{file_name}"
        entries.append(
            WolfVariantEntry(
                identity.constant_name,
                identity.resource_id,
                file_name,
                (
                    prefix,
                    f"{prefix}_tame",
                    f"{prefix}_angry",
                    f"{prefix}_baby",
                    f"{prefix}_tame_baby",
                    f"{prefix}_angry_baby",
                ),
            )
        )
    return entries


def cat_variant_entries(
    source: JavaSource, expected_count: int = 11
) -> list[CatVariantEntry]:
    identities = bootstrap_resource_key_entries(
        source,
        "CatVariant",
        {"createKey"},
        expected_count,
        registration_names={"register", "registerForAnyConditions"},
    )
    bootstrap = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, bootstrap)
    calls = [
        call
        for call in source.calls
        if call.name in {"register", "registerForAnyConditions"}
        and call.qualifier is None
        and bootstrap.body is not None
        and bootstrap.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= bootstrap.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    assets: dict[str, tuple[str, str]] = {}
    for call in calls:
        expected_arity = 5 if call.name == "register" else 4
        if (
            len(call.arguments) != expected_arity
            or call.arguments[0].canonical != context_name
        ):
            _fail(
                source,
                source.expression(call.node),
                "cat bootstrap registration shape changed",
            )
        key = call.arguments[1]
        if key.node.type != "identifier":
            _fail(source, key, "cat bootstrap resource key is not an identifier")
        assets[key.canonical] = (
            f"minecraft:{_string(source, call.arguments[2])}",
            f"minecraft:{_string(source, call.arguments[3])}",
        )

    helpers = [
        method
        for method in source.methods
        if method.name == "register" and len(method.parameters) == 5
    ]
    if len(helpers) != 1 or helpers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one five-parameter cat asset register helper"
        )
    helper = helpers[0]
    parameter_names = []
    for parameter in helper.parameters:
        name = parameter.node.child_by_field_name("name")
        if name is None:
            _fail(source, parameter, "cat asset helper parameter lacks a name")
        parameter_names.append(source.text(name))
    if parameter_names != [
        "context",
        "name",
        "adultTexture",
        "babyTexture",
        "spawnConditions",
    ]:
        _fail(source, helper.parameters[0], "cat asset helper parameters changed")
    registrations = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "context"
        and helper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= helper.body.end_byte
    ]
    expected_registration = (
        "context.register (name, new CatVariant (new ClientAsset.ResourceTexture "
        "(Identifier.withDefaultNamespace (adultTexture)), new ClientAsset.ResourceTexture "
        "(Identifier.withDefaultNamespace (babyTexture)), spawnConditions))"
    )
    if (
        len(registrations) != 1
        or source.expression(registrations[0].node).canonical != expected_registration
    ):
        expression = (
            source.expression(registrations[0].node)
            if registrations
            else source.expression(helper.node)
        )
        _fail(source, expression, "cat asset registration payload changed")

    wrappers = [
        method
        for method in source.methods
        if method.name == "registerForAnyConditions" and len(method.parameters) == 4
    ]
    if len(wrappers) != 1 or wrappers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one cat fallback registration helper"
        )
    wrapper = wrappers[0]
    wrapper_calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and wrapper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= wrapper.body.end_byte
    ]
    expected_wrapper = (
        "register (context, name, adultTexture, babyTexture, "
        "SpawnPrioritySelectors.fallback (0))"
    )
    if (
        len(wrapper_calls) != 1
        or source.expression(wrapper_calls[0].node).canonical != expected_wrapper
    ):
        expression = (
            source.expression(wrapper_calls[0].node)
            if wrapper_calls
            else source.expression(wrapper.node)
        )
        _fail(source, expression, "cat fallback registration payload changed")

    entries = []
    for identity in identities:
        asset_pair = assets.get(identity.constant_name)
        if asset_pair is None:
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: cat variant {identity.constant_name} lacks assets"
            )
        entries.append(
            CatVariantEntry(
                identity.constant_name,
                identity.resource_id,
                asset_pair[0],
                asset_pair[1],
            )
        )
    all_assets = [
        asset_id
        for entry in entries
        for asset_id in (entry.adult_asset_id, entry.baby_asset_id)
    ]
    if len(all_assets) != len(set(all_assets)):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate cat variant asset id"
        )
    return entries


def farm_animal_variant_entries(
    source: JavaSource,
    referenced_paths: dict[str, str],
    variant_type: str,
    asset_directory: str,
    expected_count: int = 3,
) -> list[FarmAnimalVariantEntry]:
    identities = bootstrap_resource_key_entries(
        source,
        variant_type,
        {"createKey"},
        expected_count,
        referenced_paths=referenced_paths,
    )
    bootstrap = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, bootstrap)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and bootstrap.body is not None
        and bootstrap.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= bootstrap.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    if len(calls) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} farm-animal "
            f"bootstrap registrations, found {len(calls)}"
        )

    facts: dict[str, tuple[str, str, str]] = {}
    model_prefix = f"{variant_type}.ModelType."
    for call in calls:
        if len(call.arguments) != 6 or call.arguments[0].canonical != context_name:
            _fail(
                source,
                source.expression(call.node),
                "farm-animal bootstrap registration shape changed",
            )
        key = call.arguments[1]
        if key.node.type != "identifier":
            _fail(source, key, "farm-animal bootstrap resource key is not an identifier")
        if key.canonical in facts:
            _fail(source, key, "duplicate farm-animal bootstrap resource key")
        model_expression = call.arguments[2]
        if not model_expression.canonical.startswith(model_prefix):
            _fail(source, model_expression, "farm-animal model type expression changed")
        model_type = model_expression.canonical.removeprefix(model_prefix)
        if not model_type.isidentifier() or model_type.upper() != model_type:
            _fail(source, model_expression, "farm-animal model type is not an enum constant")
        adult_name = _string(source, call.arguments[3])
        baby_name = _string(source, call.arguments[4])
        facts[key.canonical] = (
            model_type,
            f"minecraft:entity/{asset_directory}/{adult_name}",
            f"minecraft:entity/{asset_directory}/{baby_name}",
        )

    helpers = []
    wrappers = []
    for method in source.methods:
        if method.name != "register" or len(method.parameters) != 6:
            continue
        last_type = method.parameters[5].node.child_by_field_name("type")
        last_type_name = (
            source.expression(last_type).canonical.replace(" ", "")
            if last_type is not None
            else ""
        )
        if last_type_name == "SpawnPrioritySelectors":
            helpers.append(method)
        elif last_type_name == "TagKey<Biome>":
            wrappers.append(method)
    if len(helpers) != 1 or helpers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one farm-animal asset register helper"
        )
    helper = helpers[0]
    helper_parameter_names = []
    for parameter in helper.parameters:
        name = parameter.node.child_by_field_name("name")
        if name is None:
            _fail(source, parameter, "farm-animal asset helper parameter lacks a name")
        helper_parameter_names.append(source.text(name))
    if helper_parameter_names != [
        "context",
        "name",
        "modelType",
        "textureName",
        "babyTextureName",
        "selectors",
    ]:
        _fail(source, helper.parameters[0], "farm-animal asset helper parameters changed")

    initializers: dict[str, JavaExpression] = {}
    for node in descendants(helper.node):
        if node.type != "variable_declarator":
            continue
        name = node.child_by_field_name("name")
        value = node.child_by_field_name("value")
        if name is None or value is None:
            _fail(source, source.expression(node), "farm-animal asset local changed")
        initializers[source.text(name)] = source.expression(value)
    expected_initializers = {
        "textureId": (
            f'Identifier.withDefaultNamespace ("entity/{asset_directory}/" + textureName)'
        ),
        "babyTextureId": (
            f'Identifier.withDefaultNamespace ("entity/{asset_directory}/" + babyTextureName)'
        ),
    }
    if set(initializers) != set(expected_initializers):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: farm-animal asset helper locals changed: "
            f"expected={sorted(expected_initializers)}, actual={sorted(initializers)}"
        )
    for name, expected in expected_initializers.items():
        if initializers[name].canonical != expected:
            _fail(source, initializers[name], f"farm-animal asset formula for {name} changed")

    registrations = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "context"
        and helper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= helper.body.end_byte
    ]
    expected_registration = (
        f"context.register (name, new {variant_type} (new ModelAndTexture <> "
        "(modelType, textureId), new ClientAsset.ResourceTexture (babyTextureId), selectors))"
    )
    if (
        len(registrations) != 1
        or source.expression(registrations[0].node).canonical != expected_registration
    ):
        expression = (
            source.expression(registrations[0].node)
            if registrations
            else source.expression(helper.node)
        )
        _fail(source, expression, "farm-animal asset registration payload changed")

    if len(wrappers) != 1 or wrappers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one farm-animal biome register helper"
        )
    wrapper = wrappers[0]
    wrapper_parameter_names = []
    for parameter in wrapper.parameters:
        name = parameter.node.child_by_field_name("name")
        if name is None:
            _fail(source, parameter, "farm-animal biome helper parameter lacks a name")
        wrapper_parameter_names.append(source.text(name))
    if wrapper_parameter_names != [
        "context",
        "name",
        "modelType",
        "textureName",
        "babyTextureName",
        "spawnBiome",
    ]:
        _fail(source, wrapper.parameters[0], "farm-animal biome helper parameters changed")
    wrapper_calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and wrapper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= wrapper.body.end_byte
    ]
    expected_wrapper = (
        "register (context, name, modelType, textureName, babyTextureName, "
        "SpawnPrioritySelectors.single (new BiomeCheck (biomes), 1))"
    )
    if (
        len(wrapper_calls) != 1
        or source.expression(wrapper_calls[0].node).canonical != expected_wrapper
    ):
        expression = (
            source.expression(wrapper_calls[0].node)
            if wrapper_calls
            else source.expression(wrapper.node)
        )
        _fail(source, expression, "farm-animal biome registration payload changed")

    entries = []
    for identity in identities:
        presentation = facts.get(identity.constant_name)
        if presentation is None:
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: farm-animal variant "
                f"{identity.constant_name} lacks presentation assets"
            )
        entries.append(
            FarmAnimalVariantEntry(
                identity.constant_name,
                identity.resource_id,
                presentation[0],
                presentation[1],
                presentation[2],
            )
        )
    all_assets = [
        asset_id
        for entry in entries
        for asset_id in (entry.adult_asset_id, entry.baby_asset_id)
    ]
    if len(all_assets) != len(set(all_assets)):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate farm-animal variant asset id"
        )
    return entries


def model_and_texture_variant_entries(
    source: JavaSource,
    referenced_paths: dict[str, str],
    variant_type: str,
    asset_directory: str,
    expected_count: int,
) -> list[ModelAndTextureVariantEntry]:
    identities = bootstrap_resource_key_entries(
        source,
        variant_type,
        {"createKey"},
        expected_count,
        referenced_paths=referenced_paths,
    )
    bootstrap = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, bootstrap)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and bootstrap.body is not None
        and bootstrap.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= bootstrap.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    if len(calls) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} model-and-texture "
            f"bootstrap registrations, found {len(calls)}"
        )

    facts: dict[str, tuple[str, str]] = {}
    model_prefix = f"{variant_type}.ModelType."
    for call in calls:
        if len(call.arguments) != 5 or call.arguments[0].canonical != context_name:
            _fail(
                source,
                source.expression(call.node),
                "model-and-texture bootstrap registration shape changed",
            )
        key = call.arguments[1]
        if key.node.type != "identifier":
            _fail(
                source,
                key,
                "model-and-texture bootstrap resource key is not an identifier",
            )
        if key.canonical in facts:
            _fail(source, key, "duplicate model-and-texture bootstrap resource key")
        model_expression = call.arguments[2]
        if not model_expression.canonical.startswith(model_prefix):
            _fail(source, model_expression, "model-and-texture model type expression changed")
        model_type = model_expression.canonical.removeprefix(model_prefix)
        if not model_type.isidentifier() or model_type.upper() != model_type:
            _fail(source, model_expression, "model-and-texture model type is not an enum constant")
        texture_name = _string(source, call.arguments[3])
        facts[key.canonical] = (
            model_type,
            f"minecraft:entity/{asset_directory}/{texture_name}",
        )

    helpers = []
    wrappers = []
    for method in source.methods:
        if method.name != "register" or len(method.parameters) != 5:
            continue
        last_type = method.parameters[4].node.child_by_field_name("type")
        last_type_name = (
            source.expression(last_type).canonical.replace(" ", "")
            if last_type is not None
            else ""
        )
        if last_type_name == "SpawnPrioritySelectors":
            helpers.append(method)
        elif last_type_name == "TagKey<Biome>":
            wrappers.append(method)
    if len(helpers) != 1 or helpers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one model-and-texture asset register helper"
        )
    helper = helpers[0]
    helper_parameter_names = []
    for parameter in helper.parameters:
        name = parameter.node.child_by_field_name("name")
        if name is None:
            _fail(source, parameter, "model-and-texture helper parameter lacks a name")
        helper_parameter_names.append(source.text(name))
    if helper_parameter_names != [
        "context",
        "name",
        "modelType",
        "textureName",
        "selectors",
    ]:
        _fail(source, helper.parameters[0], "model-and-texture helper parameters changed")

    initializers: dict[str, JavaExpression] = {}
    for node in descendants(helper.node):
        if node.type != "variable_declarator":
            continue
        name = node.child_by_field_name("name")
        value = node.child_by_field_name("value")
        if name is None or value is None:
            _fail(source, source.expression(node), "model-and-texture asset local changed")
        initializers[source.text(name)] = source.expression(value)
    expected_initializer = (
        f'Identifier.withDefaultNamespace ("entity/{asset_directory}/" + textureName)'
    )
    if set(initializers) != {"textureId"}:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: model-and-texture asset helper locals changed: "
            f"expected=['textureId'], actual={sorted(initializers)}"
        )
    if initializers["textureId"].canonical != expected_initializer:
        _fail(
            source,
            initializers["textureId"],
            "model-and-texture asset formula for textureId changed",
        )

    registrations = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "context"
        and helper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= helper.body.end_byte
    ]
    expected_registration = (
        f"context.register (name, new {variant_type} (new ModelAndTexture <> "
        "(modelType, textureId), selectors))"
    )
    if (
        len(registrations) != 1
        or source.expression(registrations[0].node).canonical != expected_registration
    ):
        expression = (
            source.expression(registrations[0].node)
            if registrations
            else source.expression(helper.node)
        )
        _fail(source, expression, "model-and-texture registration payload changed")

    if len(wrappers) != 1 or wrappers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one model-and-texture biome register helper"
        )
    wrapper = wrappers[0]
    wrapper_parameter_names = []
    for parameter in wrapper.parameters:
        name = parameter.node.child_by_field_name("name")
        if name is None:
            _fail(source, parameter, "model-and-texture biome parameter lacks a name")
        wrapper_parameter_names.append(source.text(name))
    if wrapper_parameter_names != [
        "context",
        "name",
        "modelType",
        "textureName",
        "spawnBiome",
    ]:
        _fail(
            source,
            wrapper.parameters[0],
            "model-and-texture biome helper parameters changed",
        )
    wrapper_calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and wrapper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= wrapper.body.end_byte
    ]
    expected_wrapper = (
        "register (context, name, modelType, textureName, "
        "SpawnPrioritySelectors.single (new BiomeCheck (biomes), 1))"
    )
    if (
        len(wrapper_calls) != 1
        or source.expression(wrapper_calls[0].node).canonical != expected_wrapper
    ):
        expression = (
            source.expression(wrapper_calls[0].node)
            if wrapper_calls
            else source.expression(wrapper.node)
        )
        _fail(source, expression, "model-and-texture biome registration payload changed")

    entries = []
    for identity in identities:
        presentation = facts.get(identity.constant_name)
        if presentation is None:
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: model-and-texture variant "
                f"{identity.constant_name} lacks presentation assets"
            )
        entries.append(
            ModelAndTextureVariantEntry(
                identity.constant_name,
                identity.resource_id,
                presentation[0],
                presentation[1],
            )
        )
    if len({entry.asset_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate model-and-texture variant asset id"
        )
    return entries


def frog_variant_entries(
    source: JavaSource,
    referenced_paths: dict[str, str],
    expected_count: int = 3,
) -> list[FrogVariantEntry]:
    identities = bootstrap_resource_key_entries(
        source,
        "FrogVariant",
        {"createKey"},
        expected_count,
        referenced_paths=referenced_paths,
    )
    bootstrap = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, bootstrap)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and bootstrap.body is not None
        and bootstrap.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= bootstrap.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    assets: dict[str, str] = {}
    for call in calls:
        if len(call.arguments) != 4 or call.arguments[0].canonical != context_name:
            _fail(source, source.expression(call.node), "frog bootstrap registration changed")
        key = call.arguments[1]
        if key.node.type != "identifier":
            _fail(source, key, "frog bootstrap resource key is not an identifier")
        assets[key.canonical] = f"minecraft:{_string(source, call.arguments[2])}"

    helpers = []
    wrappers = []
    for method in source.methods:
        if method.name != "register" or len(method.parameters) != 4:
            continue
        last_type = method.parameters[3].node.child_by_field_name("type")
        last_type_name = (
            source.expression(last_type).canonical.replace(" ", "")
            if last_type is not None
            else ""
        )
        if last_type_name == "SpawnPrioritySelectors":
            helpers.append(method)
        elif last_type_name == "TagKey<Biome>":
            wrappers.append(method)
    if len(helpers) != 1 or helpers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one frog asset register helper"
        )
    helper = helpers[0]
    registrations = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == "context"
        and helper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= helper.body.end_byte
    ]
    expected_registration = (
        "context.register (name, new FrogVariant (new ClientAsset.ResourceTexture "
        "(Identifier.withDefaultNamespace (assetId)), selectors))"
    )
    if (
        len(registrations) != 1
        or source.expression(registrations[0].node).canonical != expected_registration
    ):
        expression = (
            source.expression(registrations[0].node)
            if registrations
            else source.expression(helper.node)
        )
        _fail(source, expression, "frog asset registration payload changed")

    if len(wrappers) != 1 or wrappers[0].body is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one frog biome registration helper"
        )
    wrapper = wrappers[0]
    wrapper_calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and wrapper.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= wrapper.body.end_byte
    ]
    expected_wrapper = (
        "register (context, name, assetId, SpawnPrioritySelectors.single "
        "(new BiomeCheck (biomes), 1))"
    )
    if (
        len(wrapper_calls) != 1
        or source.expression(wrapper_calls[0].node).canonical != expected_wrapper
    ):
        expression = (
            source.expression(wrapper_calls[0].node)
            if wrapper_calls
            else source.expression(wrapper.node)
        )
        _fail(source, expression, "frog biome registration payload changed")

    entries = []
    for identity in identities:
        asset_id = assets.get(identity.constant_name)
        if asset_id is None:
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: frog variant {identity.constant_name} lacks an asset"
            )
        entries.append(
            FrogVariantEntry(identity.constant_name, identity.resource_id, asset_id)
        )
    if len({entry.asset_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate frog variant asset id"
        )
    return entries


def identifier_constant_paths(source: JavaSource) -> dict[str, str]:
    paths: dict[str, str] = {}
    for field in source.fields:
        if field.type_spelling.replace(" ", "") != "Identifier":
            continue
        if field.initializer is None:
            _fail(source, JavaExpression("", field.span, field.node), "Identifier lacks initializer")
        _, arguments = _invocation(source, field.initializer, {"withDefaultNamespace"})
        if len(arguments) != 1:
            _fail(source, field.initializer, "Identifier initializer must have one argument")
        paths[field.name] = _string(source, arguments[0])
    if not paths:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: no Identifier constants found"
        )
    return paths


def static_register_entries(
    source: JavaSource,
    field_type: str | tuple[str, ...],
    expected_count: int,
) -> list[BootstrapEntry]:
    def matches_field_type(field: JavaField) -> bool:
        spelling = field.type_spelling.replace(" ", "")
        if isinstance(field_type, str):
            return spelling.startswith(field_type)
        return spelling in field_type

    fields = [
        field
        for field in source.fields
        if matches_field_type(field)
    ]
    entries: list[BootstrapEntry] = []
    for field in fields:
        if field.initializer is None:
            _fail(source, JavaExpression("", field.span, field.node), f"{field.name} lacks initializer")
        _, arguments = _invocation(source, field.initializer, {"register"})
        if not arguments:
            _fail(source, field.initializer, f"register field {field.name} lacks resource path")
        entries.append(BootstrapEntry(field.name, f"minecraft:{_string(source, arguments[0])}"))
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} static registrations, "
            f"extracted {len(entries)}"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(f"{source.relative_path}: duplicate constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(f"{source.relative_path}: duplicate resource id")
    return entries


_OVERWORLD_DAY_VISUAL_TRACKS = (
    ("FOG_COLOR", "ColorModifier.MULTIPLY_RGB", "i32"),
    ("SKY_COLOR", "ColorModifier.MULTIPLY_RGB", "i32"),
    ("SKY_LIGHT_COLOR", "ColorModifier.MULTIPLY_RGB", "i32"),
    ("SKY_LIGHT_FACTOR", "FloatModifier.MULTIPLY", "f32"),
    ("SKY_LIGHT_LEVEL", "FloatModifier.MULTIPLY", "f32"),
    ("SUNRISE_SUNSET_COLOR", "override", "i32"),
    ("STAR_BRIGHTNESS", "FloatModifier.MAXIMUM", "f32"),
    ("CLOUD_COLOR", "ColorModifier.MULTIPLY_ARGB", "i32"),
)


def _signed_decimal_integer(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type == "unary_expression":
        children = expression.node.named_children
        spelling = expression.canonical
        if len(children) != 1 or not spelling or spelling[0] not in "+-":
            _fail(source, expression, "expected a signed decimal integer literal")
        value = _signed_decimal_integer(source, source.expression(children[0]))
        return -value if spelling[0] == "-" else value
    if expression.node.type != "decimal_integer_literal":
        _fail(source, expression, "expected a decimal integer literal")
    try:
        return int(source.text(expression.node).replace("_", ""))
    except ValueError:
        _fail(source, expression, "invalid decimal integer literal")


def _java_float32_bits(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type != "decimal_floating_point_literal":
        _fail(source, expression, "timeline float keyframe is not a literal")
    spelling = source.text(expression.node).replace("_", "")
    if not spelling.endswith(("F", "f")):
        _fail(source, expression, "timeline float keyframe must use Java float precision")
    try:
        value = float(spelling[:-1])
        packed = struct.pack(">f", value)
    except (OverflowError, ValueError):
        _fail(source, expression, "invalid timeline float keyframe")
    if not math.isfinite(value):
        _fail(source, expression, "timeline float keyframe must be finite")
    return struct.unpack(">I", packed)[0]


def _float32(value: float) -> float:
    return struct.unpack(">f", struct.pack(">f", value))[0]


def _timeline_i32_value(
    source: JavaSource,
    expression: JavaExpression,
    fields: dict[str, JavaField],
    resolving: set[str],
) -> int:
    if expression.node.type in {"decimal_integer_literal", "unary_expression"}:
        value = _signed_decimal_integer(source, expression)
    elif expression.node.type == "identifier":
        name = expression.canonical
        field = fields.get(name)
        if field is None or field.initializer is None:
            _fail(source, expression, f"unknown timeline integer constant {name}")
        if name in resolving:
            _fail(source, expression, f"cyclic timeline integer constant {name}")
        resolving.add(name)
        value = _timeline_i32_value(source, field.initializer, fields, resolving)
        resolving.remove(name)
    elif expression.node.type == "method_invocation":
        name, arguments = _invocation(source, expression, {"colorFromFloat"})
        qualifier = expression.node.child_by_field_name("object")
        if (
            name != "colorFromFloat"
            or qualifier is None
            or source.expression(qualifier).canonical != "ARGB"
            or len(arguments) != 4
        ):
            _fail(source, expression, "unsupported timeline integer initializer")
        channels = []
        for argument in arguments:
            bits = _java_float32_bits(source, argument)
            channel = struct.unpack(">f", struct.pack(">I", bits))[0]
            channels.append(math.floor(_float32(_float32(channel) * _float32(255.0))))
        unsigned = (
            (channels[0] & 0xFF) << 24
            | (channels[1] & 0xFF) << 16
            | (channels[2] & 0xFF) << 8
            | channels[3] & 0xFF
        )
        value = unsigned if unsigned < 0x8000_0000 else unsigned - 0x1_0000_0000
    else:
        _fail(source, expression, "unsupported timeline integer value")
    if not -(1 << 31) <= value < (1 << 31):
        _fail(source, expression, "timeline integer value exceeds i32")
    return value


def overworld_day_timeline_entry(source: JavaSource) -> OverworldDayTimelineEntry:
    bootstrap = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, bootstrap)
    registrations = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is not None
        and call.qualifier.canonical == context_name
        and len(call.arguments) == 2
        and call.arguments[0].canonical == "OVERWORLD_DAY"
        and bootstrap.body is not None
        and bootstrap.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= bootstrap.body.end_byte
    ]
    if len(registrations) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one OVERWORLD_DAY registration"
        )
    timeline = registrations[0].arguments[1]
    contained_calls = [
        call
        for call in source.calls
        if timeline.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= timeline.span.end_byte
    ]
    period_calls = [call for call in contained_calls if call.name == "setPeriodTicks"]
    if len(period_calls) != 1 or len(period_calls[0].arguments) != 1:
        _fail(source, timeline, "OVERWORLD_DAY must set exactly one period")
    period_ticks = _signed_decimal_integer(source, period_calls[0].arguments[0])
    if period_ticks <= 0:
        _fail(source, period_calls[0].arguments[0], "timeline period must be positive")

    expected_by_attribute = {
        attribute: (modifier, value_kind)
        for attribute, modifier, value_kind in _OVERWORLD_DAY_VISUAL_TRACKS
    }
    track_calls = [
        call
        for call in contained_calls
        if call.name in {"addTrack", "addModifierTrack"}
        and call.arguments
        and call.arguments[0].canonical.startswith("EnvironmentAttributes.")
        and call.arguments[0].canonical.rsplit(".", 1)[-1] in expected_by_attribute
    ]
    track_calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    actual_order = [
        call.arguments[0].canonical.rsplit(".", 1)[-1] for call in track_calls
    ]
    expected_order = [attribute for attribute, _, _ in _OVERWORLD_DAY_VISUAL_TRACKS]
    if actual_order != expected_order:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: OVERWORLD_DAY visual track order mismatch: "
            f"expected={expected_order}, actual={actual_order}"
        )

    fields = {field.name: field for field in source.fields}
    tracks: list[TimelineTrackEntry] = []
    for call, attribute_name in zip(track_calls, actual_order):
        expected_modifier, value_kind = expected_by_attribute[attribute_name]
        if call.name == "addTrack":
            if len(call.arguments) != 2:
                _fail(source, source.expression(call.node), "unsupported timeline track call")
            modifier = "override"
            lambda_expression = call.arguments[1]
        else:
            if len(call.arguments) != 3:
                _fail(source, source.expression(call.node), "unsupported modifier track call")
            modifier = call.arguments[1].canonical
            lambda_expression = call.arguments[2]
        if modifier != expected_modifier:
            _fail(
                source,
                source.expression(call.node),
                f"{attribute_name} modifier changed from {expected_modifier} to {modifier}",
            )
        if lambda_expression.node.type != "lambda_expression":
            _fail(source, lambda_expression, "timeline track builder is not a lambda")
        lambda_calls = [
            candidate
            for candidate in source.calls
            if lambda_expression.span.start_byte <= candidate.span.start_byte
            and candidate.span.end_byte <= lambda_expression.span.end_byte
        ]
        if any(candidate.name != "addKeyframe" for candidate in lambda_calls):
            _fail(source, lambda_expression, "timeline visual track no longer uses default easing")
        keyframe_calls = sorted(
            lambda_calls, key=lambda candidate: (candidate.span.start_byte, candidate.span.end_byte)
        )
        keyframes: list[TimelineKeyframeEntry] = []
        for keyframe in keyframe_calls:
            if len(keyframe.arguments) != 2:
                _fail(source, source.expression(keyframe.node), "unsupported keyframe call")
            ticks = _signed_decimal_integer(source, keyframe.arguments[0])
            if ticks < 0:
                _fail(source, keyframe.arguments[0], "timeline keyframe tick must be non-negative")
            if value_kind == "i32":
                encoded_value = _timeline_i32_value(
                    source, keyframe.arguments[1], fields, set()
                )
            else:
                encoded_value = _java_float32_bits(source, keyframe.arguments[1])
            keyframes.append(TimelineKeyframeEntry(ticks, encoded_value))
        if not keyframes:
            _fail(source, lambda_expression, "timeline visual track has no keyframes")
        tracks.append(
            TimelineTrackEntry(
                attribute_name,
                modifier,
                value_kind,
                tuple(keyframes),
            )
        )

    night_sky_light = fields.get("NIGHT_SKY_LIGHT_COLOR")
    if night_sky_light is None or night_sky_light.initializer is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: NIGHT_SKY_LIGHT_COLOR is missing"
        )
    night_sky_light_color = _timeline_i32_value(
        source, night_sky_light.initializer, fields, set()
    )
    return OverworldDayTimelineEntry(
        period_ticks,
        tuple(tracks),
        night_sky_light_color,
    )


def wood_type_entries(
    source: JavaSource,
    expected_count: int = 12,
) -> list[WoodTypeEntry]:
    type_maps = [field for field in source.fields if field.name == "TYPES"]
    if (
        len(type_maps) != 1
        or type_maps[0].initializer is None
        or type_maps[0].initializer.canonical != "new Object2ObjectArrayMap <> ()"
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: unsupported WoodType registration map"
        )
    register_methods = [method for method in source.methods if method.name == "register"]
    if (
        len(register_methods) != 1
        or source.expression(register_methods[0].node).canonical
        != "private static WoodType register (final WoodType type) {TYPES.put (type.name (), type); return type;}"
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: unsupported WoodType register helper"
        )
    values_methods = [method for method in source.methods if method.name == "values"]
    if (
        len(values_methods) != 1
        or source.expression(values_methods[0].node).canonical
        != "public static Stream <WoodType> values () {return TYPES.values ().stream ();}"
    ):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: unsupported WoodType values ordering"
        )
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "") == "WoodType"
    ]
    entries: list[WoodTypeEntry] = []
    for field in fields:
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"wood type {field.name} lacks an initializer",
            )
        _, register_arguments = _invocation(source, field.initializer, {"register"})
        if len(register_arguments) != 1:
            _fail(
                source,
                field.initializer,
                f"wood type {field.name} register call must have one argument",
            )
        constructor_arguments = _object_creation_arguments(
            source, register_arguments[0], "WoodType"
        )
        if len(constructor_arguments) not in {2, 6}:
            _fail(
                source,
                register_arguments[0],
                f"wood type {field.name} has unsupported constructor arity",
            )
        entries.append(WoodTypeEntry(field.name, _string(source, constructor_arguments[0])))
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} wood types, "
            f"extracted {len(entries)}"
        )
    for attribute in ("constant_name", "name"):
        values = [getattr(entry, attribute) for entry in entries]
        if len(values) != len(set(values)):
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: duplicate wood type {attribute}"
            )
    return entries


def map_decoration_entries(
    source: JavaSource, expected_count: int = 35, *, map_colors: dict[str, int] | None = None
) -> list[MapDecorationEntry]:
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith(
            "Holder<MapDecorationType>"
        )
    ]
    entries = []
    for field in fields:
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"map decoration {field.name} lacks an initializer",
            )
        _, arguments = _invocation(source, field.initializer, {"register"})
        if len(arguments) not in {4, 6}:
            _fail(
                source,
                field.initializer,
                f"map decoration {field.name} has unsupported register arity",
            )
        map_color = None
        if len(arguments) == 6:
            color_expression = arguments[3]
            color_name = re.fullmatch(r"MapColor\.([A-Z_]+)\.col", color_expression.canonical)
            if color_name:
                if map_colors is None or color_name[1] not in map_colors:
                    _fail(source, color_expression, "map decoration color lacks its MapColor source")
                map_color = map_colors[color_name[1]]
            else:
                map_color = _signed_decimal_integer(source, color_expression)
            if map_color == -1:
                map_color = None
        entries.append(
            MapDecorationEntry(
                field.name,
                f"minecraft:{_string(source, arguments[0])}",
                f"minecraft:{_string(source, arguments[1])}",
                _boolean(source, arguments[2]),
                _boolean(source, arguments[3] if len(arguments) == 4 else arguments[4]),
                map_color,
                _boolean(source, arguments[5]) if len(arguments) == 6 else False,
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} map decorations, "
            f"extracted {len(entries)}"
        )
    for attribute in ("constant_name", "resource_id"):
        values = [getattr(entry, attribute) for entry in entries]
        if len(values) != len(set(values)):
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: duplicate map decoration {attribute}"
            )
    return entries


def map_color_constants(source: JavaSource) -> dict[str, int]:
    return {
        field.name: _decimal_integer(source, _object_creation_arguments(source, field.initializer, "MapColor")[1])
        for field in source.fields
        if field.type_spelling == "MapColor" and field.initializer is not None
    }


def _object_creation_arguments(
    source: JavaSource,
    expression: JavaExpression,
    expected_type: str,
) -> tuple[JavaExpression, ...]:
    node = expression.node
    type_node = node.child_by_field_name("type")
    arguments_node = node.child_by_field_name("arguments")
    if (
        node.type != "object_creation_expression"
        or type_node is None
        or arguments_node is None
        or source.expression(type_node).canonical.replace(" ", "") != expected_type
    ):
        _fail(source, expression, f"expected new {expected_type}(...) expression")
    return tuple(
        source.expression(node) for node in arguments_node.named_children
    )


def _decimal_integer(source: JavaSource, expression: JavaExpression) -> int:
    if expression.node.type != "decimal_integer_literal":
        _fail(source, expression, "expected a decimal integer literal")
    return int(expression.canonical.replace("_", ""), 10)


def potion_entries(
    source: JavaSource,
    known_mob_effect_constants: set[str],
    expected_count: int = 46,
    expected_effect_count: int = 45,
) -> list[PotionEntry]:
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith("Holder.Reference<Potion>")
    ]
    entries: list[PotionEntry] = []
    for field in fields:
        if field.initializer is None:
            _fail(
                source,
                JavaExpression("", field.span, field.node),
                f"{field.name} lacks initializer",
            )
        _, register_arguments = _invocation(source, field.initializer, {"register"})
        if len(register_arguments) != 2:
            _fail(source, field.initializer, "potion register must have two arguments")
        resource_id = f"minecraft:{_string(source, register_arguments[0])}"
        potion_arguments = _object_creation_arguments(
            source, register_arguments[1], "Potion"
        )
        if not potion_arguments:
            _fail(source, register_arguments[1], "Potion must carry a translation key")
        name = _string(source, potion_arguments[0])
        effects: list[PotionEffectEntry] = []
        for effect_expression in potion_arguments[1:]:
            effect_arguments = _object_creation_arguments(
                source, effect_expression, "MobEffectInstance"
            )
            if len(effect_arguments) not in {2, 3}:
                _fail(
                    source,
                    effect_expression,
                    "MobEffectInstance must have effect, duration, and optional amplifier",
                )
            effect_node = effect_arguments[0].node
            object_node = effect_node.child_by_field_name("object")
            field_node = effect_node.child_by_field_name("field")
            if (
                effect_node.type != "field_access"
                or object_node is None
                or field_node is None
                or source.expression(object_node).canonical != "MobEffects"
            ):
                _fail(
                    source,
                    effect_arguments[0],
                    "potion effect must reference a MobEffects constant",
                )
            constant_name = source.text(field_node)
            if constant_name not in known_mob_effect_constants:
                _fail(
                    source,
                    effect_arguments[0],
                    f"unknown MobEffects constant {constant_name}",
                )
            effects.append(
                PotionEffectEntry(
                    constant_name,
                    _decimal_integer(source, effect_arguments[1]),
                    _decimal_integer(source, effect_arguments[2])
                    if len(effect_arguments) == 3
                    else 0,
                )
            )
        entries.append(PotionEntry(field.name, resource_id, name, tuple(effects)))
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} potions, "
            f"extracted {len(entries)}"
        )
    if sum(len(entry.effects) for entry in entries) != expected_effect_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_effect_count} potion effects"
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(f"{source.relative_path}: duplicate potion constant")
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate potion resource id"
        )
    return entries


def trim_material_entries(
    source: JavaSource,
    material_asset_groups_source: JavaSource,
    expected_count: int = 11,
) -> list[TrimMaterialEntry]:
    asset_groups: dict[str, tuple[str, tuple[tuple[str, str], ...]]] = {}
    for field in material_asset_groups_source.fields:
        if field.type_spelling.replace(" ", "") != "MaterialAssetGroup":
            continue
        if field.initializer is None:
            _fail(
                material_asset_groups_source,
                JavaExpression("", field.span, field.node),
                f"trim material asset group {field.name} lacks initializer",
            )
        _, arguments = _invocation(
            material_asset_groups_source, field.initializer, {"create"}
        )
        if len(arguments) not in {1, 2}:
            _fail(
                material_asset_groups_source,
                field.initializer,
                "trim material asset group create call changed",
            )
        asset_name = _string(material_asset_groups_source, arguments[0])
        overrides: list[tuple[str, str]] = []
        if len(arguments) == 2:
            map_expression = arguments[1]
            map_object = map_expression.node.child_by_field_name("object")
            _, map_arguments = _invocation(
                material_asset_groups_source, map_expression, {"of"}
            )
            if (
                map_object is None
                or material_asset_groups_source.expression(map_object).canonical != "Map"
                or len(map_arguments) % 2 != 0
            ):
                _fail(
                    material_asset_groups_source,
                    map_expression,
                    "trim material override table must be Map.of(key, suffix, ...)",
                )
            for index in range(0, len(map_arguments), 2):
                equipment_asset = map_arguments[index]
                prefix = "EquipmentAssets."
                if not equipment_asset.canonical.startswith(prefix):
                    _fail(
                        material_asset_groups_source,
                        equipment_asset,
                        "trim material override key is not an EquipmentAssets constant",
                    )
                resource_id = (
                    "minecraft:"
                    + equipment_asset.canonical.removeprefix(prefix).lower()
                )
                suffix = _string(material_asset_groups_source, map_arguments[index + 1])
                overrides.append((resource_id, suffix))
        asset_groups[field.name] = (asset_name, tuple(overrides))

    resources = _declared_resource_keys(source, "TrimMaterial", {"registryKey"})
    method = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, method)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[TrimMaterialEntry] = []
    for call in calls:
        if len(call.arguments) != 4 or call.arguments[0].canonical != context_name:
            _fail(source, source.expression(call.node), "unsupported trim material register call")
        key = call.arguments[1]
        if key.node.type != "identifier" or key.canonical not in resources:
            _fail(source, key, "trim material key is not a declared identifier")
        style = call.arguments[2]
        style_object = style.node.child_by_field_name("object")
        _, style_arguments = _invocation(source, style, {"withColor"})
        if (
            style_object is None
            or source.expression(style_object).canonical != "Style.EMPTY"
            or len(style_arguments) != 1
        ):
            _fail(source, style, "trim material style must be Style.EMPTY.withColor(color)")
        color = _decimal_integer(source, style_arguments[0])
        if not 0 <= color <= 0xFF_FF_FF:
            _fail(source, style_arguments[0], "trim material color is outside 24-bit RGB")
        assets = call.arguments[3]
        assets_object = assets.node.child_by_field_name("object")
        assets_field = assets.node.child_by_field_name("field")
        if (
            assets.node.type != "field_access"
            or assets_object is None
            or assets_field is None
            or source.expression(assets_object).canonical != "MaterialAssetGroup"
            or source.text(assets_field) != key.canonical
        ):
            _fail(
                source,
                assets,
                "trim material asset group must match the registered key",
            )
        try:
            asset_name, overrides = asset_groups[key.canonical]
        except KeyError:
            _fail(source, assets, "trim material asset group definition is missing")
        entries.append(
            TrimMaterialEntry(
                key.canonical,
                resources[key.canonical],
                color,
                asset_name,
                overrides,
            )
        )
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} trim materials, "
            f"extracted {len(entries)}"
        )
    constants = [entry.constant_name for entry in entries]
    if len(constants) != len(set(constants)):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate trim material registration"
        )
    unknown = set(constants) - resources.keys()
    missing = resources.keys() - set(constants)
    if unknown or missing:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: trim material declaration/bootstrap mismatch: "
            f"unknown={sorted(unknown)}, missing={sorted(missing)}"
        )
    return entries


def decorated_pot_pattern_entries(
    source: JavaSource,
    items_source: JavaSource,
    expected_count: int = 24,
) -> list[DecoratedPotPatternEntry]:
    resources = _declared_resource_keys(
        source,
        "DecoratedPotPattern",
        {"create"},
    )
    pattern_types = [
        java_type
        for java_type in source.types
        if java_type.simple_name == "DecoratedPotPatterns"
    ]
    if len(pattern_types) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one DecoratedPotPatterns type"
        )
    map_fields = [
        field
        for field in source.fields_for(pattern_types[0])
        if field.name == "ITEM_TO_POT_TEXTURE"
    ]
    if len(map_fields) != 1 or map_fields[0].initializer is None:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected one initialized ITEM_TO_POT_TEXTURE field"
        )
    map_initializer = map_fields[0].initializer
    map_object = map_initializer.node.child_by_field_name("object")
    _, map_arguments = _invocation(source, map_initializer, {"ofEntries"})
    if map_object is None or source.expression(map_object).canonical != "Map":
        _fail(source, map_initializer, "pattern item table must use Map.ofEntries")

    item_types = [
        java_type
        for java_type in items_source.types
        if java_type.simple_name == "Items"
    ]
    if len(item_types) != 1:
        raise BootstrapCatalogExtractionError(
            f"{items_source.relative_path}: expected one Items type"
        )
    item_by_pattern: dict[str, tuple[str, str]] = {}
    for argument in map_arguments:
        entry_object = argument.node.child_by_field_name("object")
        _, entry_arguments = _invocation(source, argument, {"entry"})
        if (
            entry_object is None
            or source.expression(entry_object).canonical != "Map"
            or len(entry_arguments) != 2
        ):
            _fail(source, argument, "pattern item mapping must use Map.entry(item, key)")
        item, key = entry_arguments
        item_object = item.node.child_by_field_name("object")
        item_field = item.node.child_by_field_name("field")
        if (
            item.node.type != "field_access"
            or item_object is None
            or item_field is None
            or source.expression(item_object).canonical != "Items"
        ):
            _fail(source, item, "pattern item is not an Items constant")
        if key.node.type != "identifier" or key.canonical not in resources:
            _fail(source, key, "pattern key is not a declared identifier")
        item_constant = source.text(item_field)
        item_fields = [
            field
            for field in items_source.fields_for(item_types[0])
            if field.name == item_constant
        ]
        if len(item_fields) != 1 or item_fields[0].initializer is None:
            _fail(source, item, f"could not resolve Items.{item_constant}")
        _, item_arguments = _invocation(
            items_source,
            item_fields[0].initializer,
            {"registerItem"},
        )
        if not item_arguments:
            _fail(
                items_source,
                item_fields[0].initializer,
                f"Items.{item_constant} lacks a resource path",
            )
        item_resource = f"minecraft:{_string(items_source, item_arguments[0])}"
        if key.canonical in item_by_pattern:
            _fail(source, key, f"duplicate item mapping for {key.canonical}")
        item_by_pattern[key.canonical] = (item_constant, item_resource)

    method = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, method)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[DecoratedPotPatternEntry] = []
    for call in calls:
        if len(call.arguments) != 3 or call.arguments[0].canonical != context_name:
            _fail(source, source.expression(call.node), "unsupported pattern register call")
        key, asset = call.arguments[1:]
        if key.node.type != "identifier" or key.canonical not in resources:
            _fail(source, key, "bootstrap pattern key is not a declared identifier")
        item = item_by_pattern.get(key.canonical)
        if item is None:
            _fail(source, key, "bootstrap pattern key has no item mapping")
        entries.append(
            DecoratedPotPatternEntry(
                key.canonical,
                resources[key.canonical],
                item[0],
                item[1],
                f"minecraft:{_string(source, asset)}",
            )
        )

    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} decorated pot patterns, "
            f"extracted {len(entries)}"
        )
    constants = [entry.constant_name for entry in entries]
    if len(constants) != len(set(constants)):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate decorated pot pattern registration"
        )
    declared = set(resources)
    registered = set(constants)
    mapped = set(item_by_pattern)
    if registered != declared or mapped != declared:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: decorated pot pattern declaration/map/bootstrap mismatch: "
            f"bootstrap_missing={sorted(declared - registered)}, "
            f"map_missing={sorted(declared - mapped)}, "
            f"unknown={sorted((registered | mapped) - declared)}"
        )
    for attribute in ("resource_id", "item_resource_id", "asset_id"):
        values = [getattr(entry, attribute) for entry in entries]
        if len(values) != len(set(values)):
            raise BootstrapCatalogExtractionError(
                f"{source.relative_path}: duplicate decorated pot pattern {attribute}"
            )
    blanks = [entry for entry in entries if entry.constant_name == "BLANK"]
    if len(blanks) != 1 or (
        blanks[0].resource_id,
        blanks[0].item_resource_id,
        blanks[0].asset_id,
    ) != ("minecraft:blank", "minecraft:brick", "minecraft:decorated_pot_side"):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: BLANK must map brick to decorated_pot_side"
        )
    return entries


def painting_variant_entries(
    source: JavaSource, expected_count: int = 51
) -> list[PaintingVariantEntry]:
    resources = _declared_resource_keys(source, "PaintingVariant", {"create"})
    method = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, method)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[PaintingVariantEntry] = []
    for call in calls:
        if len(call.arguments) not in {4, 5} or call.arguments[0].canonical != context_name:
            _fail(source, source.expression(call.node), "unsupported painting register call")
        key = call.arguments[1]
        if key.node.type != "identifier" or key.canonical not in resources:
            _fail(source, key, "painting resource key is not a declared identifier")
        dimensions: list[int] = []
        for argument in call.arguments[2:4]:
            if argument.node.type != "decimal_integer_literal":
                _fail(source, argument, "painting dimension is not a decimal integer literal")
            dimensions.append(int(argument.canonical.replace("_", ""), 10))
        author = call.arguments[4].canonical if len(call.arguments) == 5 else "true"
        if author not in {"true", "false"}:
            _fail(source, call.arguments[4], "painting author flag is not a boolean literal")
        entries.append(
            PaintingVariantEntry(
                key.canonical,
                resources[key.canonical],
                dimensions[0],
                dimensions[1],
                author == "true",
            )
        )
    constants = [entry.constant_name for entry in entries]
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} painting variants, "
            f"extracted {len(entries)}"
        )
    if len(constants) != len(set(constants)):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate painting bootstrap registration"
        )
    unknown = set(constants) - resources.keys()
    missing = resources.keys() - set(constants)
    if unknown or missing:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: painting declaration/bootstrap mismatch: "
            f"unknown={sorted(unknown)}, missing={sorted(missing)}"
        )
    return entries


def banner_pattern_entries(
    source: JavaSource, expected_count: int = 43
) -> list[BannerPatternEntry]:
    entries = bootstrap_resource_key_entries(
        source, "BannerPattern", {"create"}, expected_count
    )
    helpers = [method for method in source.methods if method.name == "register"]
    if len(helpers) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one banner register helper"
        )
    constructors = [
        node
        for node in descendants(helpers[0].node)
        if node.type == "object_creation_expression"
        and (type_node := node.child_by_field_name("type")) is not None
        and source.expression(type_node).canonical == "BannerPattern"
    ]
    if len(constructors) != 1:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected exactly one BannerPattern constructor"
        )
    arguments = constructors[0].child_by_field_name("arguments")
    if arguments is None or len(arguments.named_children) != 2:
        _fail(
            source,
            source.expression(constructors[0]),
            "BannerPattern constructor must have two arguments",
        )
    asset = source.expression(arguments.named_children[0])
    expected_asset = "key.identifier ()"
    if asset.canonical != expected_asset:
        _fail(
            source,
            asset,
            f"unsupported banner asset expression {asset.canonical}",
        )
    translation = source.expression(arguments.named_children[1])
    expected = '"block.minecraft.banner." + key.identifier ().toShortLanguageKey ()'
    if translation.canonical != expected:
        _fail(
            source,
            translation,
            f"unsupported banner translation expression {translation.canonical}",
        )
    return [
        BannerPatternEntry(
            entry.constant_name,
            entry.resource_id,
            entry.resource_id,
            "block.minecraft.banner."
            + entry.resource_id.removeprefix("minecraft:"),
        )
        for entry in entries
    ]


def enchantment_entries(
    source: JavaSource, expected_count: int = 43
) -> list[EnchantmentEntry]:
    resources = _declared_resource_keys(source, "Enchantment", {"key"})
    method = _bootstrap_method(source)
    context_name = _bootstrap_context_name(source, method)
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[EnchantmentEntry] = []
    for call in calls:
        if len(call.arguments) != 3 or call.arguments[0].canonical != context_name:
            _fail(source, source.expression(call.node), "unsupported enchantment register call")
        key = call.arguments[1]
        if key.node.type != "identifier" or key.canonical not in resources:
            _fail(source, key, "enchantment resource key is not a declared identifier")
        value = call.arguments[2]
        definitions = [
            candidate
            for candidate in source.calls
            if candidate.name == "definition"
            and value.span.start_byte <= candidate.span.start_byte
            and candidate.span.end_byte <= value.span.end_byte
        ]
        if len(definitions) != 1 or len(definitions[0].arguments) < 3:
            _fail(source, value, "expected exactly one Enchantment.definition call")
        definition_arguments = definitions[0].arguments
        has_primary_items = definition_arguments[1].node.type != "decimal_integer_literal"
        max_level_index = 3 if has_primary_items else 2
        if len(definition_arguments) <= max_level_index:
            _fail(source, value, "Enchantment.definition lacks a max-level argument")
        max_level = definition_arguments[max_level_index]
        if max_level.node.type != "decimal_integer_literal":
            _fail(source, max_level, "enchantment max level is not an integer literal")
        entries.append(
            EnchantmentEntry(
                key.canonical,
                resources[key.canonical],
                int(max_level.canonical.replace("_", ""), 10),
            )
        )
    constants = [entry.constant_name for entry in entries]
    if len(entries) != expected_count:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} enchantments, "
            f"extracted {len(entries)}"
        )
    if len(constants) != len(set(constants)):
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: duplicate enchantment bootstrap registration"
        )
    unknown = set(constants) - resources.keys()
    missing = resources.keys() - set(constants)
    if unknown or missing:
        raise BootstrapCatalogExtractionError(
            f"{source.relative_path}: enchantment declaration/bootstrap mismatch: "
            f"unknown={sorted(unknown)}, missing={sorted(missing)}"
        )
    return entries
