"""Tree-sitter-backed domain extraction for vanilla core registry catalogs."""

from __future__ import annotations

import json
import math
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaField, JavaMethod, JavaSource


class CoreRegistryExtractionError(ValueError):
    """Explicit unsupported or inconsistent core registry source."""


@dataclass(frozen=True)
class RegistryEntry:
    constant_name: str
    resource_id: str
    damage_effect: str | None = None


@dataclass(frozen=True)
class EntityTypeEntry:
    constant_name: str
    resource_id: str
    allowed_in_peaceful: bool
    only_op_can_set_nbt: bool
    serializable: bool
    width: float
    height: float
    eye_height: float | None
    passenger_attachments: tuple[tuple[float, float, float], ...]


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


def _float_literal(source: JavaSource, expression: JavaExpression, description: str) -> float:
    if expression.node.type == "unary_expression":
        children = expression.node.named_children
        spelling = expression.canonical
        if len(children) != 1 or not spelling or spelling[0] not in "+-":
            _fail(source, expression, f"{description} is not a signed numeric literal")
        value = _float_literal(source, source.expression(children[0]), description)
        return -value if spelling[0] == "-" else value
    if expression.node.type not in {
        "decimal_floating_point_literal",
        "decimal_integer_literal",
        "hex_floating_point_literal",
        "hex_integer_literal",
    }:
        _fail(source, expression, f"{description} is not a numeric literal")
    spelling = expression.canonical.replace("_", "").removesuffix("F").removesuffix("f")
    spelling = spelling.removesuffix("D").removesuffix("d")
    try:
        value = float.fromhex(spelling) if spelling.lower().startswith("0x") else float(spelling)
    except ValueError as error:
        _fail(source, expression, f"invalid {description} literal: {error}")
    if not math.isfinite(value):
        _fail(source, expression, f"{description} literal is not finite")
    return value


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
    registered: list[tuple[str, str | None]] = []
    for call in calls:
        if call.qualifier is None or call.qualifier.canonical != "context":
            continue
        if len(call.arguments) < 2 or call.arguments[0].node.type != "identifier":
            _fail(source, source.expression(call.node), "damage bootstrap register is unsupported")
        registered.append(
            (
                call.arguments[0].canonical,
                _damage_type_effect(source, call.arguments[1]),
            )
        )
    registered_names = [name for name, _ in registered]
    if len(registered_names) != len(set(registered_names)):
        raise CoreRegistryExtractionError("duplicate damage type bootstrap registration")
    unknown = set(registered_names) - resources.keys()
    missing = resources.keys() - set(registered_names)
    if not registered or unknown or missing:
        raise CoreRegistryExtractionError(
            f"damage declaration/bootstrap mismatch: unknown={sorted(unknown)}, "
            f"missing={sorted(missing)}"
        )
    return [RegistryEntry(name, resources[name], effect) for name, effect in registered]


def _damage_type_effect(
    source: JavaSource, expression: JavaExpression
) -> str | None:
    if expression.node.type != "object_creation_expression":
        return None
    type_node = expression.node.child_by_field_name("type")
    arguments = expression.node.child_by_field_name("arguments")
    if type_node is None or source.text(type_node) != "DamageType" or arguments is None:
        _fail(source, expression, "damage bootstrap value is not a DamageType constructor")
    effect_names = {
        "DamageEffects.HURT": "hurt",
        "DamageEffects.THORNS": "thorns",
        "DamageEffects.DROWNING": "drowning",
        "DamageEffects.BURNING": "burning",
        "DamageEffects.POKING": "poking",
        "DamageEffects.FREEZING": "freezing",
    }
    effects = [
        effect_names[argument.canonical]
        for argument in (source.expression(node) for node in arguments.named_children)
        if argument.canonical in effect_names
    ]
    if len(effects) > 1:
        _fail(source, expression, "damage constructor has multiple effects")
    return effects[0] if effects else "hurt"


def entity_type_entries(
    source: JavaSource, expected_count: int = 157
) -> list[EntityTypeEntry]:
    op_only_constants = _constant_set_members(source, "OP_ONLY_CUSTOM_DATA")
    entries: list[EntityTypeEntry] = []
    for field in _fields_with_type(source, ("EntityType<",)):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, f"entity register has {len(arguments)} arguments")
        sized_calls = _calls_in_expression(source, field.initializer, "sized")
        if len(sized_calls) != 1:
            _fail(
                source,
                field.initializer,
                f"entity builder expected one sized call, found {len(sized_calls)}",
            )
        sized_call = sized_calls[0]
        if len(sized_call.arguments) != 2:
            _fail(source, source.expression(sized_call.node), "sized expects two arguments")
        width = _float_literal(source, sized_call.arguments[0], "entity width")
        height = _float_literal(source, sized_call.arguments[1], "entity height")
        if width < 0.0 or height < 0.0:
            _fail(source, source.expression(sized_call.node), "entity dimensions are negative")
        eye_height_calls = _calls_in_expression(source, field.initializer, "eyeHeight")
        if len(eye_height_calls) > 1:
            _fail(source, field.initializer, "entity builder has multiple eyeHeight calls")
        eye_height = None
        if eye_height_calls:
            call = eye_height_calls[0]
            if len(call.arguments) != 1:
                _fail(source, source.expression(call.node), "eyeHeight expects one argument")
            eye_height = _float_literal(source, call.arguments[0], "eyeHeight")
        passenger_attachment_calls = _calls_in_expression(
            source, field.initializer, "passengerAttachments"
        )
        if len(passenger_attachment_calls) > 1:
            _fail(
                source,
                field.initializer,
                "entity builder has multiple passengerAttachments calls",
            )
        passenger_attachments: list[tuple[float, float, float]] = []
        if passenger_attachment_calls:
            call = passenger_attachment_calls[0]
            if not call.arguments:
                _fail(source, source.expression(call.node), "passengerAttachments is empty")
            argument_kinds = {argument.node.type for argument in call.arguments}
            if argument_kinds == {"object_creation_expression"}:
                for argument in call.arguments:
                    type_node = argument.node.child_by_field_name("type")
                    arguments_node = argument.node.child_by_field_name("arguments")
                    if (
                        type_node is None
                        or source.text(type_node).replace(" ", "").split(".")[-1] != "Vec3"
                        or arguments_node is None
                        or len(arguments_node.named_children) != 3
                    ):
                        _fail(source, argument, "passenger attachment is not new Vec3(x, y, z)")
                    passenger_attachments.append(
                        tuple(
                            _float_literal(
                                source,
                                source.expression(component),
                                "passenger attachment coordinate",
                            )
                            for component in arguments_node.named_children
                        )
                    )
            elif not any(kind == "object_creation_expression" for kind in argument_kinds):
                passenger_attachments.extend(
                    (0.0, _float_literal(source, argument, "passenger attachment y"), 0.0)
                    for argument in call.arguments
                )
            else:
                _fail(source, source.expression(call.node), "mixed passenger attachment overload")
        entries.append(
            EntityTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                not _calls_in_expression(source, field.initializer, "notInPeaceful"),
                field.name in op_only_constants,
                not _calls_in_expression(source, field.initializer, "noSave"),
                width,
                height,
                eye_height,
                tuple(passenger_attachments),
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


def common_item_component_constant_names(
    source: JavaSource, expected_count: int = 10
) -> list[str]:
    fields = [field for field in source.fields if field.name == "COMMON_ITEM_COMPONENTS"]
    if len(fields) != 1:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected one COMMON_ITEM_COMPONENTS field, found {len(fields)}"
        )
    field = fields[0]
    initializer = field.initializer
    if initializer is None:
        _fail(
            source,
            JavaExpression("", field.span, field.node),
            "COMMON_ITEM_COMPONENTS lacks an initializer",
        )
    build_calls = [
        call
        for call in _calls_in_expression(source, initializer, "build")
        if call.span == initializer.span
    ]
    if len(build_calls) != 1:
        _fail(
            source,
            initializer,
            "COMMON_ITEM_COMPONENTS is not terminated by the root builder build call",
        )
    set_calls = _calls_in_expression(source, initializer, "set")
    set_calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    members: list[str] = []
    for call in set_calls:
        if len(call.arguments) != 2:
            _fail(
                source,
                source.expression(call.node),
                f"COMMON_ITEM_COMPONENTS set has {len(call.arguments)} arguments",
            )
        component = call.arguments[0]
        if component.node.type != "identifier":
            _fail(
                source,
                component,
                "COMMON_ITEM_COMPONENTS member is not a data component constant",
            )
        members.append(component.canonical)
    if len(members) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} common item components, extracted {len(members)}"
        )
    if len(members) != len(set(members)):
        _fail(source, initializer, "COMMON_ITEM_COMPONENTS contains duplicate constants")
    known_components = {
        field.name for field in _fields_with_type(source, ("DataComponentType<",))
    }
    unknown = set(members) - known_components
    if unknown:
        _fail(
            source,
            initializer,
            f"COMMON_ITEM_COMPONENTS references unknown component constants: {sorted(unknown)}",
        )
    return members


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
