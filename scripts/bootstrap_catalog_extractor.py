"""Tree-sitter-backed extraction for vanilla bootstrap resource catalogs."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import (
    JavaExpression,
    JavaField,
    JavaMethod,
    JavaSource,
    descendants,
    type_body_members,
)


class BootstrapCatalogExtractionError(ValueError):
    """Explicit unsupported or inconsistent bootstrap source."""


@dataclass(frozen=True)
class BootstrapEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class BannerPatternEntry:
    constant_name: str
    resource_id: str
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
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    registered: list[str] = []
    for call in calls:
        if len(call.arguments) < 2 or call.arguments[0].canonical != context_name:
            _fail(source, source.expression(call.node), "unsupported bootstrap register call")
        key = call.arguments[1]
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
    field_type: str,
    expected_count: int,
) -> list[BootstrapEntry]:
    fields = [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith(field_type)
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
