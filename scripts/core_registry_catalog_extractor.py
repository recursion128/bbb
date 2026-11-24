"""Tree-sitter-backed domain extraction for vanilla core registry catalogs."""

from __future__ import annotations

import json
import math
from collections.abc import Sequence
from dataclasses import dataclass
from enum import Enum

from sound_event_catalog_extractor import direct_sound_event_entries
from vanilla_java_source import (
    JavaExpression,
    JavaField,
    JavaMethod,
    JavaSource,
    JavaType,
    resolve_class_hierarchy,
    type_body_members,
)


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
    default_loot_table: str | None
    mob_category: str
    allowed_in_peaceful: bool
    only_op_can_set_nbt: bool
    serializable: bool
    can_summon: bool
    fire_immune: bool
    width: float
    height: float
    eye_height: float | None
    name_tag_offset: float | None
    passenger_attachments: tuple[tuple[float, float, float], ...]
    vehicle_attachment: tuple[float, float, float]
    client_tracking_range: int
    update_interval: int
    can_spawn_far_from_player: bool


@dataclass(frozen=True)
class BlockEntityTypeEntry:
    constant_name: str
    resource_id: str
    only_op_can_set_nbt: bool
    provides_menu: bool


@dataclass(frozen=True)
class DataComponentTypeEntry:
    constant_name: str
    resource_id: str
    value_type: str
    persistent: bool
    persistent_codec: str | None
    network_codec: str
    network_codec_source: str
    cache_encoding: bool
    ignore_swap_animation: bool


@dataclass(frozen=True)
class AnimalVariantComponentRegistryEntry:
    component_constant_name: str
    registry_constant_name: str
    registry_resource_id: str
    value_type_fqcn: str


@dataclass(frozen=True)
class NoteBlockInstrumentEntry:
    constant_name: str
    serialized_name: str
    sound_constant_name: str
    sound_resource_id: str
    instrument_type: str


@dataclass(frozen=True)
class ParticleTypeEntry:
    constant_name: str
    resource_id: str
    override_limiter: bool
    codec_family: VanillaParticleCodecFamily


class VanillaParticleCodecFamily(Enum):
    SIMPLE = "simple"
    BLOCK = "block"
    POWER = "power"
    DUST = "dust"
    DUST_COLOR_TRANSITION = "dust_color_transition"
    SPELL = "spell"
    COLOR = "color"
    SCULK_CHARGE = "sculk_charge"
    ITEM = "item"
    VIBRATION = "vibration"
    TRAIL = "trail"
    SHRIEK = "shriek"


@dataclass(frozen=True)
class PoiTypeEntry:
    constant_name: str
    resource_id: str
    max_tickets: int
    valid_range: int
    matching_blocks: tuple[str, ...] = ()
    head_only: bool = False


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise CoreRegistryExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _plain_string_literal(source: JavaSource, expression: JavaExpression) -> str:
    """A Java string literal's value, with no namespace assumed."""

    if expression.node.type != "string_literal":
        _fail(source, expression, "registry resource id is not a string literal")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid registry resource string: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "registry resource id is empty or non-string")
    return value


def _string_literal(source: JavaSource, expression: JavaExpression) -> str:
    return "minecraft:" + _plain_string_literal(source, expression)


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


def _int_literal(source: JavaSource, expression: JavaExpression, description: str) -> int:
    if expression.node.type == "unary_expression":
        children = expression.node.named_children
        spelling = expression.canonical
        if len(children) != 1 or not spelling or spelling[0] not in "+-":
            _fail(source, expression, f"{description} is not a signed integer literal")
        value = _int_literal(source, source.expression(children[0]), description)
        return -value if spelling[0] == "-" else value
    if expression.node.type not in {"decimal_integer_literal", "hex_integer_literal"}:
        _fail(source, expression, f"{description} is not an integer literal")
    spelling = expression.canonical.replace("_", "").removesuffix("L").removesuffix("l")
    try:
        return int(spelling, 0)
    except ValueError as error:
        _fail(source, expression, f"invalid {description} literal: {error}")


# java.lang.Integer.MAX_VALUE is fixed by the Java language spec, not by vanilla
# sources, so the only JDK constant the builder chains use is spelled out here.
_JDK_INT_CONSTANTS = {"Integer.MAX_VALUE": 2**31 - 1}


def _int_expression(source: JavaSource, expression: JavaExpression, description: str) -> int:
    constant = _JDK_INT_CONSTANTS.get(expression.canonical.replace(" ", ""))
    if constant is not None:
        return constant
    return _int_literal(source, expression, description)


def _builder_int_option(
    source: JavaSource,
    initializer: JavaExpression,
    method_name: str,
    description: str,
    default: int,
) -> int:
    """Read a single-argument builder setter, falling back to the Builder field default."""
    calls = _calls_in_expression(source, initializer, method_name)
    if len(calls) > 1:
        _fail(source, initializer, f"entity builder has multiple {method_name} calls")
    if not calls:
        return default
    call = calls[0]
    if len(call.arguments) != 1:
        _fail(source, source.expression(call.node), f"{method_name} expects one argument")
    return _int_expression(source, call.arguments[0], description)


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


def entity_type_track_delta_exclusions(
    source: JavaSource, expected_count: int = 10
) -> frozenset[str]:
    methods = [method for method in source.methods if method.name == "trackDeltas"]
    if len(methods) != 1:
        _fail(
            source,
            source.expression(source.root_node),
            f"expected one trackDeltas method, found {len(methods)}",
        )
    method = methods[0]
    method_expression = source.expression(method.node)
    return_type = method.node.child_by_field_name("type")
    parameters = method.node.child_by_field_name("parameters")
    if (
        return_type is None
        or source.text(return_type) != "boolean"
        or parameters is None
        or parameters.named_children
    ):
        _fail(source, method_expression, "trackDeltas does not have the expected signature")
    body = method.node.child_by_field_name("body")
    if body is None or len(body.named_children) != 1:
        _fail(source, method_expression, "trackDeltas body is not a single return")
    return_statement = body.named_children[0]
    if return_statement.type != "return_statement" or len(return_statement.named_children) != 1:
        _fail(
            source,
            source.expression(return_statement),
            "trackDeltas body is not a single return",
        )
    predicate = return_statement.named_children[0]
    exclusions: list[tuple[str, JavaExpression]] = []

    def extract(node) -> None:
        expression = source.expression(node)
        if node.type != "binary_expression":
            _fail(source, expression, "trackDeltas predicate is not a this != CONSTANT && chain")
        left = node.child_by_field_name("left")
        operator = node.child_by_field_name("operator")
        right = node.child_by_field_name("right")
        if left is None or operator is None or right is None:
            _fail(source, expression, "trackDeltas predicate is incomplete")
        spelling = source.text(operator)
        if spelling == "&&":
            extract(left)
            extract(right)
            return
        if (
            spelling != "!="
            or left.type != "this"
            or right.type != "identifier"
        ):
            _fail(source, expression, "trackDeltas predicate is not a this != CONSTANT && chain")
        exclusions.append((source.text(right), source.expression(right)))

    extract(predicate)
    seen: set[str] = set()
    for constant, expression in exclusions:
        if constant in seen:
            _fail(source, expression, f"trackDeltas repeats entity type constant {constant}")
        seen.add(constant)
    known_constants = {
        field.name for field in _fields_with_type(source, ("EntityType<",))
    }
    for constant, expression in exclusions:
        if constant not in known_constants:
            _fail(source, expression, f"trackDeltas references unknown entity type {constant}")
    if len(exclusions) != expected_count:
        _fail(
            source,
            source.expression(predicate),
            f"expected {expected_count} trackDeltas exclusions, extracted {len(exclusions)}",
        )
    return frozenset(seen)


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


def number_format_type_entries(source: JavaSource) -> list[RegistryEntry]:
    bootstraps = [method for method in source.methods if method.name == "bootstrap"]
    if len(bootstraps) != 1:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected one bootstrap method, found {len(bootstraps)}"
        )
    calls = _calls_in_method(source, bootstraps[0], "register")
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[RegistryEntry] = []
    for call in calls:
        if len(call.arguments) != 3:
            _fail(
                source,
                source.expression(call.node),
                "number format register expects three arguments",
            )
        resource_id = _string_literal(source, call.arguments[1])
        constant_name = resource_id.removeprefix("minecraft:").upper()
        entries.append(RegistryEntry(constant_name, resource_id))
    if len(entries) != 3:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected 3 number format types, found {len(entries)}"
        )
    _validate_unique(entries, "number format type")
    return entries


def _poi_matching_states(source: JavaSource, expression: JavaExpression) -> tuple[tuple[str, ...], bool]:
    def block_constant(value: JavaExpression) -> str:
        if value.node.type != "field_access" or not value.canonical.startswith("Blocks."):
            _fail(source, value, "POI state set expected a Blocks constant")
        name = value.canonical.removeprefix("Blocks.")
        if not name.isidentifier():
            _fail(source, value, "invalid POI block constant")
        return name

    if expression.node.type == "method_invocation":
        calls = _calls_in_expression(source, expression, "getBlockStates")
        if len(calls) != 1 or len(calls[0].arguments) != 1 or calls[0].span != expression.span or calls[0].qualifier is not None:
            _fail(source, expression, "POI state set expected getBlockStates(Blocks.constant)")
        return (block_constant(calls[0].arguments[0]),), False
    if expression.node.type != "identifier":
        _fail(source, expression, "unknown POI state set expression")
    fields = [field for field in source.fields if field.name == expression.canonical]
    if len(fields) != 1 or fields[0].initializer is None:
        _fail(source, expression, "POI state set lacks its definition")
    initializer = fields[0].initializer
    calls = _calls_in_expression(source, initializer, "of")
    if len(calls) != 1 or calls[0].qualifier is None or calls[0].qualifier.canonical != "ImmutableList":
        _fail(source, initializer, "POI state set expected ImmutableList.of")
    blocks = tuple(block_constant(argument) for argument in calls[0].arguments)
    if not blocks or len(set(blocks)) != len(blocks):
        _fail(source, initializer, "POI state set must contain distinct blocks")
    base = "ImmutableList.of(" + ",".join("Blocks." + block for block in blocks) + ")"
    base += ".stream().flatMap(block->block.getStateDefinition().getPossibleStates().stream())"
    end = ".collect(ImmutableSet.toImmutableSet())"
    actual = "".join(initializer.canonical.split())
    if actual == base + end:
        return blocks, False
    if actual == base + ".filter(state->state.getValue(BedBlock.PART)==BedPart.HEAD)" + end:
        return blocks, True
    _fail(source, initializer, "unknown POI state-set transformation")


def poi_type_entries(source: JavaSource, expected_count: int = 21, *, include_states: bool = False) -> list[PoiTypeEntry]:
    if include_states:
        helpers = [method for method in source.methods if method.name == "getBlockStates"]
        if len(helpers) != 1 or helpers[0].body is None or "".join(source.text(helpers[0].body).split()) != "{returnImmutableSet.copyOf(block.getStateDefinition().getPossibleStates());}":
            raise CoreRegistryExtractionError("POI getBlockStates no longer returns every state")
    resources: dict[str, str] = {}
    for field in _fields_with_type(source, ("ResourceKey<PoiType>",)):
        initializer = field.initializer
        if initializer is None:
            _fail(source, JavaExpression("", field.span, field.node), "POI key lacks initializer")
        calls = _calls_in_expression(source, initializer, "createKey")
        if len(calls) != 1 or len(calls[0].arguments) != 1:
            _fail(source, initializer, f"POI key {field.name} expected createKey(string)")
        resources[field.name] = _string_literal(source, calls[0].arguments[0])

    bootstraps = [method for method in source.methods if method.name == "bootstrap"]
    if len(bootstraps) != 1:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected one POI bootstrap method, found {len(bootstraps)}"
        )
    calls = _calls_in_method(source, bootstraps[0], "register")
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[PoiTypeEntry] = []
    registered_names: list[str] = []
    for call in calls:
        if len(call.arguments) != 5 or call.arguments[1].node.type != "identifier":
            _fail(source, source.expression(call.node), "POI register expects registry, key, states, maxTickets, validRange")
        constant_name = call.arguments[1].canonical
        resource_id = resources.get(constant_name)
        if resource_id is None:
            _fail(source, call.arguments[1], f"unknown POI key {constant_name}")
        max_tickets = _int_literal(source, call.arguments[3], "POI maxTickets")
        valid_range = _int_literal(source, call.arguments[4], "POI validRange")
        if max_tickets < 0 or valid_range < 0:
            _fail(source, source.expression(call.node), "POI limits must be non-negative")
        registered_names.append(constant_name)
        blocks, head_only = _poi_matching_states(source, call.arguments[2]) if include_states else ((), False)
        entries.append(PoiTypeEntry(constant_name, resource_id, max_tickets, valid_range, blocks, head_only))

    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected {expected_count} POI types, found {len(entries)}"
        )
    if len(registered_names) != len(set(registered_names)):
        raise CoreRegistryExtractionError("duplicate POI type bootstrap registration")
    unknown_or_missing = set(resources).symmetric_difference(registered_names)
    if unknown_or_missing:
        raise CoreRegistryExtractionError(
            f"POI declaration/bootstrap mismatch: {sorted(unknown_or_missing)}"
        )
    _validate_unique(entries, "POI type")
    return entries


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


def vec3_constant(source: JavaSource, field_name: str) -> tuple[float, float, float]:
    matches = [
        field
        for field in source.fields
        if field.name == field_name and field.type_spelling.replace(" ", "").endswith("Vec3")
    ]
    if len(matches) != 1 or matches[0].initializer is None:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected one initialized Vec3 field {field_name}"
        )
    expression = matches[0].initializer
    type_node = expression.node.child_by_field_name("type")
    arguments_node = expression.node.child_by_field_name("arguments")
    if (
        expression.node.type != "object_creation_expression"
        or type_node is None
        or source.text(type_node).replace(" ", "").split(".")[-1] != "Vec3"
        or arguments_node is None
        or len(arguments_node.named_children) != 3
    ):
        _fail(source, expression, f"{field_name} is not new Vec3(x, y, z)")
    return tuple(
        _float_literal(
            source,
            source.expression(component),
            f"{field_name} coordinate",
        )
        for component in arguments_node.named_children
    )


def entity_type_entries(
    source: JavaSource,
    expected_count: int = 157,
    named_vec3_constants: dict[str, tuple[float, float, float]] | None = None,
) -> list[EntityTypeEntry]:
    named_vec3_constants = named_vec3_constants or {}
    op_only_constants = _constant_set_members(source, "OP_ONLY_CUSTOM_DATA")
    entries: list[EntityTypeEntry] = []
    for field in _fields_with_type(source, ("EntityType<",)):
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(source, field.initializer, f"entity register has {len(arguments)} arguments")
        builder_calls = _calls_in_expression(source, field.initializer, "of")
        category_argument_index = 1
        if not builder_calls:
            builder_calls = _calls_in_expression(source, field.initializer, "createNothing")
            category_argument_index = 0
        if (
            len(builder_calls) != 1
            or len(builder_calls[0].arguments) != category_argument_index + 1
        ):
            _fail(
                source,
                field.initializer,
                "entity builder expected one of(factory, MobCategory) or createNothing(MobCategory) call",
            )
        category_argument = builder_calls[0].arguments[category_argument_index]
        category = category_argument.canonical
        category_prefix = "MobCategory."
        if not category.startswith(category_prefix):
            _fail(
                source,
                category_argument,
                "entity builder category is not a MobCategory constant",
            )
        mob_category = category.removeprefix(category_prefix)
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
        name_tag_offset_calls = _calls_in_expression(
            source, field.initializer, "nameTagOffset"
        )
        if len(name_tag_offset_calls) > 1:
            _fail(source, field.initializer, "entity builder has multiple nameTagOffset calls")
        name_tag_offset = None
        if name_tag_offset_calls:
            call = name_tag_offset_calls[0]
            if len(call.arguments) != 1:
                _fail(source, source.expression(call.node), "nameTagOffset expects one argument")
            name_tag_offset = _float_literal(
                source, call.arguments[0], "nameTagOffset"
            )
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
        vehicle_attachment_calls = _calls_in_expression(
            source, field.initializer, "vehicleAttachment"
        )
        riding_offset_calls = _calls_in_expression(source, field.initializer, "ridingOffset")
        if len(vehicle_attachment_calls) > 1 or len(riding_offset_calls) > 1:
            _fail(source, field.initializer, "entity builder has multiple vehicle attachments")
        if vehicle_attachment_calls and riding_offset_calls:
            _fail(
                source,
                field.initializer,
                "entity builder mixes vehicleAttachment and ridingOffset",
            )
        vehicle_attachment = (0.0, 0.0, 0.0)
        if riding_offset_calls:
            call = riding_offset_calls[0]
            if len(call.arguments) != 1:
                _fail(source, source.expression(call.node), "ridingOffset expects one argument")
            vehicle_attachment = (
                0.0,
                -_float_literal(source, call.arguments[0], "ridingOffset"),
                0.0,
            )
        elif vehicle_attachment_calls:
            call = vehicle_attachment_calls[0]
            if len(call.arguments) != 1:
                _fail(
                    source,
                    source.expression(call.node),
                    "vehicleAttachment expects one argument",
                )
            argument = call.arguments[0]
            vehicle_attachment = named_vec3_constants.get(argument.canonical)
            if vehicle_attachment is None:
                _fail(
                    source,
                    argument,
                    "vehicleAttachment is not a resolved named Vec3 constant",
                )
        # EntityType.java:1615 defaults clientTrackingRange to 5 chunks and
        # EntityType.java:1616 defaults updateInterval to 3 ticks when the builder
        # chain omits the setters.
        client_tracking_range = _builder_int_option(
            source,
            field.initializer,
            "clientTrackingRange",
            "clientTrackingRange",
            default=5,
        )
        update_interval = _builder_int_option(
            source,
            field.initializer,
            "updateInterval",
            "updateInterval",
            default=3,
        )
        if client_tracking_range < 0:
            _fail(source, field.initializer, "clientTrackingRange is negative")
        if update_interval < 1:
            _fail(source, field.initializer, "updateInterval is not positive")
        # EntityType.java:1630 seeds canSpawnFarFromPlayer from the mob category and
        # EntityType.java:1714 lets the builder chain force it on.
        can_spawn_far_from_player = mob_category in {"CREATURE", "MISC"} or bool(
            _calls_in_expression(source, field.initializer, "canSpawnFarFromPlayer")
        )
        entries.append(
            EntityTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                None
                if _calls_in_expression(source, field.initializer, "noLootTable")
                else "minecraft:entities/" + _plain_string_literal(source, arguments[0]),
                mob_category,
                not _calls_in_expression(source, field.initializer, "notInPeaceful"),
                field.name in op_only_constants,
                not _calls_in_expression(source, field.initializer, "noSave"),
                not _calls_in_expression(source, field.initializer, "noSummon"),
                bool(_calls_in_expression(source, field.initializer, "fireImmune")),
                width,
                height,
                eye_height,
                name_tag_offset,
                tuple(passenger_attachments),
                vehicle_attachment,
                client_tracking_range,
                update_interval,
                can_spawn_far_from_player,
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


MENU_PROVIDER_INTERFACE = "MenuProvider"


def _block_entity_class_name(source: JavaSource, field: JavaField) -> str:
    spelling = field.type_spelling.replace(" ", "")
    if not spelling.startswith("BlockEntityType<") or not spelling.endswith(">"):
        _fail(source, field.initializer, f"unparsable block entity type {field.type_spelling}")
    return spelling[len("BlockEntityType<") : -1].split(".")[-1]


def block_entity_menu_provider_classes(
    block_entity_class_sources: Sequence[JavaSource],
) -> frozenset[str]:
    types = [item for source in block_entity_class_sources for item in source.types]
    resolve_class_hierarchy(types)
    by_simple_name: dict[str, list] = {}
    for item in types:
        by_simple_name.setdefault(item.simple_name, []).append(item)

    def declares_menu_provider(simple_name: str) -> bool:
        seen: set[str] = set()
        pending = [simple_name]
        while pending:
            current = pending.pop()
            if current in seen:
                continue
            seen.add(current)
            for item in by_simple_name.get(current, ()):
                for spelling in item.interface_spellings:
                    if spelling.split(".")[-1].split("<")[0] == MENU_PROVIDER_INTERFACE:
                        return True
                if item.superclass_spelling is not None:
                    pending.append(item.superclass_spelling.split(".")[-1].split("<")[0])
        return False

    return frozenset(name for name in by_simple_name if declares_menu_provider(name))


def block_entity_type_entries(
    source: JavaSource,
    menu_provider_classes: frozenset[str],
    expected_count: int = 49,
    expected_menu_provider_count: int = 14,
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
                _block_entity_class_name(source, field) in menu_provider_classes,
            )
        )
    menu_providers = sum(entry.provides_menu for entry in entries)
    if menu_providers != expected_menu_provider_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_menu_provider_count} menu-provider block entity types, "
            f"extracted {menu_providers}"
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
        type_spelling = "".join(field.type_spelling.split())
        type_prefix = "DataComponentType<"
        if not type_spelling.startswith(type_prefix) or not type_spelling.endswith(">"):
            _fail(source, field.initializer, "component field has an unsupported value type")
        persistent_calls = _calls_in_expression(source, field.initializer, "persistent")
        network_calls = _calls_in_expression(
            source, field.initializer, "networkSynchronized"
        )
        cache_calls = _calls_in_expression(source, field.initializer, "cacheEncoding")
        ignore_swap_calls = _calls_in_expression(
            source, field.initializer, "ignoreSwapAnimation"
        )
        for name, calls, argument_count in (
            ("persistent", persistent_calls, 1),
            ("networkSynchronized", network_calls, 1),
            ("cacheEncoding", cache_calls, 0),
            ("ignoreSwapAnimation", ignore_swap_calls, 0),
        ):
            if len(calls) > 1:
                _fail(source, field.initializer, f"component builder repeats {name}")
            if calls and len(calls[0].arguments) != argument_count:
                _fail(
                    source,
                    field.initializer,
                    f"component builder {name} has {len(calls[0].arguments)} arguments",
                )
        persistent_codec = (
            persistent_calls[0].arguments[0].canonical if persistent_calls else None
        )
        if network_calls:
            network_codec = network_calls[0].arguments[0].canonical
            network_codec_source = "explicit"
        elif persistent_codec is not None:
            network_codec = persistent_codec
            network_codec_source = "persistent-fallback"
        else:
            _fail(
                source,
                field.initializer,
                "component has neither a persistent nor explicit network codec",
            )
        entries.append(
            DataComponentTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                type_spelling[len(type_prefix) : -1],
                persistent_codec is not None,
                persistent_codec,
                network_codec,
                network_codec_source,
                bool(cache_calls),
                bool(ignore_swap_calls),
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} data component types, extracted {len(entries)}"
        )
    _validate_unique(entries, "data component type")
    return entries


def animal_variant_component_registry_entries(
    component_source: JavaSource,
    registries_source: JavaSource,
    expected_count: int = 12,
) -> list[AnimalVariantComponentRegistryEntry]:
    animal_package = "net.minecraft.world.entity.animal."
    registry_by_value_type: dict[str, tuple[str, str]] = {}
    for field in registries_source.fields:
        spelling = field.type_spelling.replace(" ", "")
        prefix = "ResourceKey<Registry<"
        if not spelling.startswith(prefix) or not spelling.endswith(">>"):
            continue
        simple_type = spelling[len(prefix) : -2]
        value_type = registries_source.imports.get(simple_type)
        if value_type is None or not value_type.startswith(animal_package):
            continue
        initializer = field.initializer
        if initializer is None:
            _fail(
                registries_source,
                JavaExpression("", field.span, field.node),
                f"animal variant registry {field.name} lacks an initializer",
            )
        calls = _calls_in_expression(registries_source, initializer, "createRegistryKey")
        if len(calls) != 1 or calls[0].span != initializer.span or len(calls[0].arguments) != 1:
            _fail(
                registries_source,
                initializer,
                f"animal variant registry {field.name} is not one createRegistryKey call",
            )
        resource_id = _string_literal(registries_source, calls[0].arguments[0])
        if value_type in registry_by_value_type:
            _fail(
                registries_source,
                initializer,
                f"duplicate animal variant registry value type {value_type}",
            )
        registry_by_value_type[value_type] = (field.name, resource_id)

    entries: list[AnimalVariantComponentRegistryEntry] = []
    for field in _fields_with_type(component_source, ("DataComponentType<Holder<",)):
        spelling = field.type_spelling.replace(" ", "")
        prefix = "DataComponentType<Holder<"
        if not spelling.endswith(">>"):
            expression = field.initializer or JavaExpression("", field.span, field.node)
            _fail(component_source, expression, "malformed holder data component type")
        simple_type = spelling[len(prefix) : -2]
        value_type = component_source.imports.get(simple_type)
        if value_type is None or not value_type.startswith(animal_package):
            continue
        registry = registry_by_value_type.get(value_type)
        if registry is None:
            expression = field.initializer or JavaExpression("", field.span, field.node)
            _fail(
                component_source,
                expression,
                f"animal variant component {field.name} lacks a matching registry",
            )
        entries.append(
            AnimalVariantComponentRegistryEntry(
                field.name, registry[0], registry[1], value_type
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} animal variant component registries, "
            f"extracted {len(entries)}"
        )
    for attribute in (
        "component_constant_name",
        "registry_constant_name",
        "registry_resource_id",
        "value_type_fqcn",
    ):
        values = [getattr(entry, attribute) for entry in entries]
        if len(values) != len(set(values)):
            raise CoreRegistryExtractionError(
                f"duplicate animal variant component registry {attribute}"
            )
    return entries


def animal_variant_texture_registry_entries(
    entries: list[AnimalVariantComponentRegistryEntry],
    value_type_sources: dict[str, JavaSource],
    expected_count: int = 7,
) -> list[AnimalVariantComponentRegistryEntry]:
    texture_dependencies = {
        "net.minecraft.core.ClientAsset",
        "net.minecraft.world.entity.variant.ModelAndTexture",
    }
    result: list[AnimalVariantComponentRegistryEntry] = []
    for entry in entries:
        source = value_type_sources.get(entry.value_type_fqcn)
        if source is None:
            raise CoreRegistryExtractionError(
                f"missing animal variant value source {entry.value_type_fqcn}"
            )
        if not any(value_type.fqcn == entry.value_type_fqcn for value_type in source.types):
            raise CoreRegistryExtractionError(
                f"{source.relative_path}: does not declare {entry.value_type_fqcn}"
            )
        network_codecs = [
            field
            for field in source.fields
            if field.owner == entry.value_type_fqcn and field.name == "NETWORK_CODEC"
        ]
        if len(network_codecs) != 1 or network_codecs[0].initializer is None:
            raise CoreRegistryExtractionError(
                f"{source.relative_path}: {entry.value_type_fqcn} must declare one initialized NETWORK_CODEC"
            )

        texture_aliases = {
            simple_name
            for simple_name, fqcn in source.imports.items()
            if fqcn in texture_dependencies
        }
        if not texture_aliases:
            continue

        local_fields: dict[str, JavaField] = {}
        package_prefix = source.package + "." if source.package else ""
        for field in source.fields:
            if field.initializer is None:
                continue
            relative_owner = field.owner.removeprefix(package_prefix)
            owner_parts = relative_owner.split(".")
            for owner_spelling in (
                relative_owner,
                owner_parts[-1],
                ".".join(owner_parts[1:]),
            ):
                if owner_spelling:
                    local_fields[f"{owner_spelling}.{field.name}"] = field

        def expression_uses_texture_codec(
            expression: JavaExpression, seen_fields: set[tuple[str, str]]
        ) -> bool:
            calls = [
                call
                for call in source.calls
                if expression.span.start_byte <= call.span.start_byte
                and call.span.end_byte <= expression.span.end_byte
            ]
            for call in calls:
                qualifier = call.qualifier
                if qualifier is None:
                    continue
                if qualifier.canonical.split(".", 1)[0] in texture_aliases:
                    return True
            for call in calls:
                qualifier = call.qualifier
                if qualifier is None:
                    continue
                field = local_fields.get(qualifier.canonical)
                if field is None or field.initializer is None:
                    continue
                key = (field.owner, field.name)
                if key in seen_fields:
                    continue
                seen_fields.add(key)
                if expression_uses_texture_codec(field.initializer, seen_fields):
                    return True
            return False

        network_codec = network_codecs[0]
        if not expression_uses_texture_codec(
            network_codec.initializer, {(network_codec.owner, network_codec.name)}
        ):
            _fail(
                source,
                network_codec.initializer,
                "NETWORK_CODEC imports client texture types without a reachable texture codec",
            )
        result.append(entry)

    if len(result) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} animal texture variant registries, "
            f"extracted {len(result)}"
        )
    return result


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
    option_families = {
        "BlockParticleOption": VanillaParticleCodecFamily.BLOCK,
        "PowerParticleOption": VanillaParticleCodecFamily.POWER,
        "DustParticleOptions": VanillaParticleCodecFamily.DUST,
        "DustColorTransitionOptions": VanillaParticleCodecFamily.DUST_COLOR_TRANSITION,
        "SpellParticleOption": VanillaParticleCodecFamily.SPELL,
        "ColorParticleOption": VanillaParticleCodecFamily.COLOR,
        "SculkChargeParticleOptions": VanillaParticleCodecFamily.SCULK_CHARGE,
        "ItemParticleOption": VanillaParticleCodecFamily.ITEM,
        "VibrationParticleOption": VanillaParticleCodecFamily.VIBRATION,
        "TrailParticleOption": VanillaParticleCodecFamily.TRAIL,
        "ShriekParticleOption": VanillaParticleCodecFamily.SHRIEK,
    }
    entries: list[ParticleTypeEntry] = []
    for field in _fields_with_type(source, ("SimpleParticleType", "ParticleType<")):
        arguments = _register_arguments(source, field)
        if len(arguments) < 2:
            _fail(source, field.initializer, f"particle register has {len(arguments)} arguments")
        boolean = arguments[1].canonical
        if boolean not in {"true", "false"}:
            _fail(source, arguments[1], "particle override-limiter is not a boolean literal")
        compact_type = field.type_spelling.replace(" ", "")
        if compact_type == "SimpleParticleType":
            if len(arguments) != 2:
                _fail(source, field.initializer, "simple particle register must have two arguments")
            codec_family = VanillaParticleCodecFamily.SIMPLE
        else:
            if not compact_type.startswith("ParticleType<") or not compact_type.endswith(">"):
                _fail(source, field.initializer, "unsupported particle field type")
            option_type = compact_type[len("ParticleType<") : -1]
            codec_family = option_families.get(option_type)
            if codec_family is None:
                _fail(source, field.initializer, f"unknown particle option codec owner {option_type}")
            if len(arguments) != 4:
                _fail(source, field.initializer, "option particle register must have four arguments")
            if option_type not in arguments[2].canonical.replace(" ", ""):
                _fail(
                    source,
                    arguments[2],
                    f"particle codec factory does not reference generic owner {option_type}",
                )
        entries.append(
            ParticleTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
                boolean == "true",
                codec_family,
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} particle types, extracted {len(entries)}"
        )
    _validate_unique(entries, "particle type")
    return entries


def activity_entries(source: JavaSource, expected_count: int = 26) -> list[RegistryEntry]:
    entries: list[RegistryEntry] = []
    for field in _fields_with_type(source, ("Activity",)):
        arguments = _register_arguments(source, field)
        if len(arguments) != 1:
            _fail(source, field.initializer, "activity register must have one argument")
        entries.append(RegistryEntry(field.name, _string_literal(source, arguments[0])))
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"expected {expected_count} activities, extracted {len(entries)}"
        )
    _validate_unique(entries, "activity")
    return entries


def _enum_type(source: JavaSource, simple_name: str) -> JavaType:
    types = [
        java_type
        for java_type in source.types
        if java_type.kind == "enum" and java_type.simple_name == simple_name
    ]
    if len(types) != 1:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected exactly one {simple_name} enum, found {len(types)}"
        )
    return types[0]


def _enum_constant_names(source: JavaSource, java_type: JavaType) -> list[str]:
    names: list[str] = []
    for node in type_body_members(java_type.body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        if name is None:
            _fail(source, source.expression(node), "enum constant lacks a name")
        names.append(source.text(name))
    if not names or len(names) != len(set(names)):
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: {java_type.simple_name} has no or duplicate constants"
        )
    return names


def mob_category_entries(
    source: JavaSource, expected_count: int = 8
) -> list[tuple[str, str]]:
    """Extract ``MobCategory`` constants and serialized names in enum order."""

    entries: list[tuple[str, str]] = []
    for node in type_body_members(_enum_type(source, "MobCategory").body_node):
        if node.type != "enum_constant":
            continue
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None or not arguments.named_children:
            _fail(source, source.expression(node), "mob category constant is incomplete")
        serialized_name = source.expression(arguments.named_children[0])
        entries.append((source.text(name), _plain_string_literal(source, serialized_name)))
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected {expected_count} mob categories, "
            f"extracted {len(entries)}"
        )
    if len({name for name, _ in entries}) != len(entries) or len(
        {serialized for _, serialized in entries}
    ) != len(entries):
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: duplicate mob category constant or serialized name"
        )
    return entries


def note_block_instrument_type_names(source: JavaSource) -> list[str]:
    """`NoteBlockInstrument.Type`'s constants in declaration order."""

    return _enum_constant_names(source, _enum_type(source, "Type"))


def _qualified_tail(source: JavaSource, expression: JavaExpression, description: str) -> str:
    if expression.node.type not in {"field_access", "identifier"}:
        _fail(source, expression, f"{description} is not a constant reference")
    return expression.canonical.replace(" ", "").rsplit(".", 1)[-1]


def note_block_instrument_entries(
    source: JavaSource,
    sound_events_source: JavaSource,
    expected_count: int = 27,
    expected_direct_sound_count: int = 1808,
) -> list[NoteBlockInstrumentEntry]:
    """`NoteBlockInstrument`'s constants, joined to the sound events they name.

    Each constant carries three constructor arguments: the serialized name, a
    `SoundEvents` constant and a `NoteBlockInstrument.Type`. The sound constant
    is resolved through the same `SoundEvents.java` registration walk the
    sound-event registry snapshot uses, so the resource id is derived from the
    registration and never spelled out here.
    """

    type_names = set(note_block_instrument_type_names(source))
    sound_resource_ids = {
        entry.constant_name: entry.resource_id
        for entry in direct_sound_event_entries(
            sound_events_source, expected_direct_sound_count
        )
    }
    entries: list[NoteBlockInstrumentEntry] = []
    for node in type_body_members(_enum_type(source, "NoteBlockInstrument").body_node):
        if node.type != "enum_constant":
            continue
        expression = source.expression(node)
        name = node.child_by_field_name("name")
        arguments = node.child_by_field_name("arguments")
        if name is None or arguments is None or len(arguments.named_children) != 3:
            _fail(source, expression, "instrument constant expects name, sound event and type")
        constant_name = source.text(name)
        serialized_name = _plain_string_literal(
            source, source.expression(arguments.named_children[0])
        )
        sound_expression = source.expression(arguments.named_children[1])
        sound_constant_name = _qualified_tail(
            source, sound_expression, f"{constant_name} sound event"
        )
        if not sound_expression.canonical.replace(" ", "").startswith("SoundEvents."):
            _fail(source, sound_expression, f"{constant_name} sound is not a SoundEvents constant")
        sound_resource_id = sound_resource_ids.get(sound_constant_name)
        if sound_resource_id is None:
            _fail(
                source,
                sound_expression,
                f"SoundEvents.{sound_constant_name} is not a registered sound event",
            )
        type_expression = source.expression(arguments.named_children[2])
        instrument_type = _qualified_tail(source, type_expression, f"{constant_name} type")
        if instrument_type not in type_names:
            _fail(source, type_expression, f"{constant_name} names an unknown Type")
        entries.append(
            NoteBlockInstrumentEntry(
                constant_name,
                serialized_name,
                sound_constant_name,
                sound_resource_id,
                instrument_type,
            )
        )
    if len(entries) != expected_count:
        raise CoreRegistryExtractionError(
            f"{source.relative_path}: expected {expected_count} note block instruments, "
            f"extracted {len(entries)}"
        )
    constants = [entry.constant_name for entry in entries]
    serialized = [entry.serialized_name for entry in entries]
    if len(constants) != len(set(constants)):
        raise CoreRegistryExtractionError("duplicate note block instrument constant")
    if len(serialized) != len(set(serialized)):
        raise CoreRegistryExtractionError("duplicate note block instrument serialized name")
    return entries
