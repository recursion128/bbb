"""Tree-sitter-backed domain extraction for vanilla core registry catalogs."""

from __future__ import annotations

import json
import math
from dataclasses import dataclass

from collections.abc import Sequence

from vanilla_java_source import (
    JavaExpression,
    JavaField,
    JavaMethod,
    JavaSource,
    resolve_class_hierarchy,
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
        entries.append(
            EntityTypeEntry(
                field.name,
                _string_literal(source, arguments[0]),
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
