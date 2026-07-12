"""Tree-sitter-backed domain extraction for vanilla core registry catalogs."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaField, JavaMethod, JavaSource


class CoreRegistryExtractionError(ValueError):
    """Explicit unsupported or inconsistent core registry source."""


@dataclass(frozen=True)
class RegistryEntry:
    constant_name: str
    resource_id: str


@dataclass(frozen=True)
class EntityTypeEntry:
    constant_name: str
    resource_id: str
    allowed_in_peaceful: bool
    only_op_can_set_nbt: bool
    serializable: bool


@dataclass(frozen=True)
class BlockEntityTypeEntry:
    constant_name: str
    resource_id: str
    only_op_can_set_nbt: bool


@dataclass(frozen=True)
class DataComponentTypeEntry:
    constant_name: str
    resource_id: str
    persistent: bool


@dataclass(frozen=True)
class ParticleTypeEntry:
    constant_name: str
    resource_id: str
    override_limiter: bool


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise CoreRegistryExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string_literal(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "registry resource id is not a string literal")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid registry resource string: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "registry resource id is empty or non-string")
    return "minecraft:" + value


def _register_arguments(source: JavaSource, field: JavaField) -> tuple[JavaExpression, ...]:
    initializer = field.initializer
    if initializer is None or initializer.node.type != "method_invocation":
        expression = initializer or JavaExpression("", field.span, field.node)
        _fail(source, expression, f"registry field {field.name} lacks a register call")
    name = initializer.node.child_by_field_name("name")
    arguments = initializer.node.child_by_field_name("arguments")
    if name is None or arguments is None or source.text(name) != "register":
        _fail(source, initializer, f"registry field {field.name} is not initialized by register")
    return tuple(source.expression(node) for node in arguments.named_children)


def _calls_in_expression(source: JavaSource, expression: JavaExpression, name: str) -> list:
    return [
        call
        for call in source.calls
        if call.name == name
        and expression.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= expression.span.end_byte
    ]


def _calls_in_method(source: JavaSource, method: JavaMethod, name: str) -> list:
    if method.body is None:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: method {method.name} lacks a body"
        )
    return [
        call
        for call in source.calls
        if call.name == name
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]


def _validate_unique(entries: list, description: str) -> None:
    constants = [entry.constant_name for entry in entries]
    resources = [entry.resource_id for entry in entries]
    if len(constants) != len(set(constants)):
        raise CoreRegistryExtractionError(f"duplicate {description} constant")
    if len(resources) != len(set(resources)):
        raise CoreRegistryExtractionError(f"duplicate {description} resource id")


def _fields_with_type(source: JavaSource, prefixes: tuple[str, ...]) -> list[JavaField]:
    return [
        field
        for field in source.fields
        if field.type_spelling.replace(" ", "").startswith(prefixes)
    ]


def _constant_set_members(source: JavaSource, field_name: str) -> set[str]:
    fields = [field for field in source.fields if field.name == field_name]
    if len(fields) != 1:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected one {field_name} field, found {len(fields)}"
        )
    field = fields[0]
    initializer = field.initializer
    if initializer is None:
        _fail(
            source,
            JavaExpression("", field.span, field.node),
            f"{field_name} lacks an initializer",
        )
    calls = _calls_in_expression(source, initializer, "of")
    calls = [
        call
        for call in calls
        if call.qualifier is not None and call.qualifier.canonical == "Set"
    ]
    if len(calls) != 1 or calls[0].span != initializer.span:
        _fail(source, initializer, f"{field_name} is not initialized by Set.of")
    members: list[str] = []
    for argument in calls[0].arguments:
        if argument.node.type != "identifier":
            _fail(source, argument, f"{field_name} member is not a registry constant")
        members.append(argument.canonical)
    if len(members) != len(set(members)):
        _fail(source, initializer, f"{field_name} contains duplicate constants")
    return set(members)


def damage_type_entries(source: JavaSource) -> list[RegistryEntry]:
    resources: dict[str, str] = {}
    fields = _fields_with_type(source, ("ResourceKey<DamageType>",))
    for field in fields:
        initializer = field.initializer
        if initializer is None:
            _fail(source, JavaExpression("", field.span, field.node), "damage key lacks initializer")
        calls = _calls_in_expression(source, initializer, "withDefaultNamespace")
        if len(calls) != 1 or len(calls[0].arguments) != 1:
            _fail(
                source,
                initializer,
                f"damage key {field.name} expected one withDefaultNamespace(string) call",
            )
        resource_id = _string_literal(source, calls[0].arguments[0])
        previous = resources.setdefault(field.name, resource_id)
        if previous != resource_id:
            _fail(source, initializer, f"conflicting damage key {field.name}")
    if not resources:
        raise CoreRegistryExtractionError(f"{source.relative_path}: no damage type keys found")

    bootstraps = [method for method in source.methods if method.name == "bootstrap"]
    if len(bootstraps) != 1:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected one bootstrap method, found {len(bootstraps)}"
        )
    calls = _calls_in_method(source, bootstraps[0], "register")
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    registered: list[str] = []
    for call in calls:
        if call.qualifier is None or call.qualifier.canonical != "context":
            continue
        if len(call.arguments) < 2 or call.arguments[0].node.type != "identifier":
            _fail(source, source.expression(call.node), "damage bootstrap register is unsupported")
        registered.append(call.arguments[0].canonical)
    if len(registered) != len(set(registered)):
        raise CoreRegistryExtractionError("duplicate damage type bootstrap registration")
    unknown = set(registered) - resources.keys()
    missing = resources.keys() - set(registered)
    if not registered or unknown or missing:
        raise CoreRegistryExtractionError(
            f"damage declaration/bootstrap mismatch: unknown={sorted(unknown)}, "
            f"missing={sorted(missing)}"
        )
    return [RegistryEntry(name, resources[name]) for name in registered]


def entity_type_entries(
    source: JavaSource, expected_count: int = 157
) -> list[EntityTypeEntry]:
    op_only_constants = _constant_set_members(source, "OP_ONLY_CUSTOM_DATA")
    entries: list[EntityTypeEntry] = []
    for field in _fields_with_type(source, ("EntityType<",)):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, f"entity register has {len(arguments)} arguments")
        entries.append(
            EntityTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                not _calls_in_expression(source, field.initializer, "notInPeaceful"),
                field.name in op_only_constants,
                not _calls_in_expression(source, field.initializer, "noSave"),
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} entity types, extracted {len(entries)}"
        )
    _validate_unique(entries, "entity type")
    unknown = op_only_constants - {entry.constant_name for entry in entries}
    if unknown:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: OP_ONLY_CUSTOM_DATA references unknown entity types: "
            f"{sorted(unknown)}"
        )
    return entries


def block_entity_type_entries(
    source: JavaSource, expected_count: int = 49
) -> list[BlockEntityTypeEntry]:
    op_only_constants = _constant_set_members(source, "OP_ONLY_CUSTOM_DATA")
    entries: list[BlockEntityTypeEntry] = []
    for field in _fields_with_type(source, ("BlockEntityType<",)):
        arguments = _register_arguments(source, field)
        if len(arguments) < 2:
            _fail(source, field.initializer, f"block entity register has {len(arguments)} arguments")
        entries.append(
            BlockEntityTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                field.name in op_only_constants,
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} block entity types, extracted {len(entries)}"
        )
    _validate_unique(entries, "block entity type")
    unknown = op_only_constants - {entry.constant_name for entry in entries}
    if unknown:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: OP_ONLY_CUSTOM_DATA references unknown block entity types: "
            f"{sorted(unknown)}"
        )
    return entries


def data_component_predicate_type_entries(
    source: JavaSource, expected_count: int = 15
) -> list[RegistryEntry]:
    entries: list[RegistryEntry] = []
    for field in _fields_with_type(source, ("DataComponentPredicate.Type<",)):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, f"predicate register has {len(arguments)} arguments")
        entries.append(RegistryEntry(field.name, _string_literal(source, arguments[0])))
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} component predicate types, extracted {len(entries)}"
        )
    _validate_unique(entries, "data component predicate type")
    return entries


def data_component_type_entries(
    source: JavaSource, expected_count: int = 110
) -> list[DataComponentTypeEntry]:
    entries: list[DataComponentTypeEntry] = []
    for field in _fields_with_type(source, ("DataComponentType<",)):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, f"component register has {len(arguments)} arguments")
        entries.append(
            DataComponentTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                bool(_calls_in_expression(source, field.initializer, "persistent")),
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} data component types, extracted {len(entries)}"
        )
    _validate_unique(entries, "data component type")
    return entries


def particle_type_entries(
    source: JavaSource, expected_count: int = 117
) -> list[ParticleTypeEntry]:
    entries: list[ParticleTypeEntry] = []
    for field in _fields_with_type(source, ("SimpleParticleType", "ParticleType<")):
        arguments = _register_arguments(source, field)
        if len(arguments) < 2:
            _fail(source, field.initializer, f"particle register has {len(arguments)} arguments")
        boolean = arguments[1].canonical
        if boolean not in {"true", "false"}:
            _fail(source, arguments[1], "particle override-limiter is not a boolean literal")
        entries.append(
            ParticleTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                boolean == "true",
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} particle types, extracted {len(entries)}"
        )
    _validate_unique(entries, "particle type")
    return entries
