"""Vanilla 26.1 entity metadata extraction over the shared Java CST frontend.

This layer recognizes Minecraft metadata declarations only.  Java lexing,
delimiter recovery, declarations, calls, and byte spans are owned exclusively
by :mod:`vanilla_java_source` and Tree-sitter Java.
"""

from __future__ import annotations

import hashlib
import math
import struct
from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Iterable, Sequence

from tree_sitter import Node

from vanilla_java_source import (
    JavaCall,
    JavaField,
    JavaSource,
    JavaSourceError,
    JavaType,
    SourceSpan,
    canonical_expression,
    descendants,
    enclosing_type,
    parse_source,
    resolve_class_hierarchy,
    resolve_type_reference,
)


ENTITY_TREE = Path("net/minecraft/world/entity")
ENTITY_TYPE_SOURCE = ENTITY_TREE / "EntityType.java"
SERIALIZER_SOURCE = Path("net/minecraft/network/syncher/EntityDataSerializers.java")
DIRECTION_SOURCE = Path("net/minecraft/core/Direction.java")
REGISTRIES_SOURCE = Path("net/minecraft/core/registries/Registries.java")
WEATHERING_COPPER_SOURCE = Path("net/minecraft/world/level/block/WeatheringCopper.java")
TNT_BLOCK_SOURCE = Path("net/minecraft/world/level/block/TntBlock.java")
DYE_COLOR_SOURCE = Path("net/minecraft/world/item/DyeColor.java")
GAME_RENDERER_SOURCE = Path("net/minecraft/client/renderer/GameRenderer.java")
BASE_SPAWNER_SOURCE = Path("net/minecraft/world/level/BaseSpawner.java")
SOUND_EVENTS_SOURCE = Path("net/minecraft/sounds/SoundEvents.java")
SUPPORTING_SOURCES = (
    SERIALIZER_SOURCE,
    DIRECTION_SOURCE,
    REGISTRIES_SOURCE,
    WEATHERING_COPPER_SOURCE,
    TNT_BLOCK_SOURCE,
    DYE_COLOR_SOURCE,
    GAME_RENDERER_SOURCE,
    BASE_SPAWNER_SOURCE,
    SOUND_EVENTS_SOURCE,
)


class ExtractionError(JavaSourceError):
    """Explicit entity metadata extraction diagnostic."""


@dataclass
class Accessor:
    owner: str
    java_name: str
    serializer: str
    declaration_order: int
    source_path: str
    declaration_span: SourceSpan
    data_id: int = -1
    default_expression: str | None = None
    default_span: SourceSpan | None = None
    default_node: Node | None = field(default=None, repr=False)


@dataclass(frozen=True)
class Flag:
    owner: str
    java_name: str
    expression: str
    value: int | None
    accessor_name: str | None
    status: str
    reason: str | None
    source_path: str
    source_span: SourceSpan


@dataclass(frozen=True)
class EntityTypeRegistration:
    entity_type_id: int
    constant_name: str
    resource_id: str
    runtime_class: str | None
    diagnostic: str | None
    source_span: SourceSpan
    attribute_factory_owner: str | None = None
    attribute_factory_method: str | None = None
    attribute_constants: tuple[str, ...] = ()
    attribute_base_overrides: tuple[tuple[str, int | None], ...] = ()


@dataclass
class ExtractionResult:
    classes: list[JavaType]
    accessors: list[Accessor]
    flags: list[Flag]
    entity_types: list[EntityTypeRegistration]
    serializer_names: list[str]
    manifest_digest: str
    sources: dict[str, JavaSource] = field(repr=False)


DEFAULT_ATTRIBUTES_OWNER = "net.minecraft.world.entity.ai.attributes.DefaultAttributes"
ATTRIBUTE_SUPPLIER_OWNER = "net.minecraft.world.entity.ai.attributes.AttributeSupplier"
ATTRIBUTES_OWNER = "net.minecraft.world.entity.ai.attributes.Attributes"


def _ancestor(node: Node, kind: str) -> Node | None:
    current: Node | None = node
    while current is not None:
        if current.type == kind:
            return current
        current = current.parent
    return None


def _field_for_call(source: JavaSource, call: JavaCall) -> JavaField:
    declarator = _ancestor(call.node, "variable_declarator")
    if declarator is None:
        raise ExtractionError(
            f"{source.relative_path}:{call.span.start_point[0] + 1}: "
            "defineId is not a field initializer"
        )
    matches = [
        item
        for item in source.fields
        if item.span.start_byte == declarator.start_byte
        and item.span.end_byte == declarator.end_byte
    ]
    if len(matches) != 1:
        raise ExtractionError(
            f"{source.relative_path}:{call.span.start_point[0] + 1}: "
            f"defineId field resolution produced {len(matches)} matches"
        )
    return matches[0]


def _resolve_owner(spelling: str, enclosing: JavaType, known: dict[str, JavaType]) -> str:
    if spelling == enclosing.simple_name or enclosing.fqcn.endswith("." + spelling):
        return enclosing.fqcn
    resolved = resolve_type_reference(spelling, enclosing, known)
    if resolved is None:
        raise ExtractionError(
            f"{enclosing.source_path}: cannot resolve defineId owner {spelling}"
        )
    return resolved


def extract_accessors(
    source: JavaSource,
    known_types: dict[str, JavaType],
) -> list[Accessor]:
    accessors: list[Accessor] = []
    for call in source.calls:
        if call.name != "defineId":
            continue
        qualifier = call.qualifier.canonical if call.qualifier else None
        if qualifier is None or qualifier.rsplit(".", 1)[-1] != "SynchedEntityData":
            raise ExtractionError(
                f"{source.relative_path}:{call.span.start_point[0] + 1}: "
                f"unsupported defineId qualifier {qualifier!r}"
            )
        if len(call.arguments) != 2:
            raise ExtractionError(
                f"{source.relative_path}:{call.span.start_point[0] + 1}: "
                "defineId requires exactly two arguments"
            )
        owner_expression = call.arguments[0].canonical.replace(" ", "")
        if not owner_expression.endswith(".class"):
            raise ExtractionError(
                f"{source.relative_path}:{call.span.start_point[0] + 1}: "
                f"unsupported defineId owner {owner_expression!r}"
            )
        serializer_expression = call.arguments[1].canonical.replace(" ", "")
        serializer_owner, separator, serializer_name = serializer_expression.rpartition(".")
        if not separator or serializer_owner.rsplit(".", 1)[-1] != "EntityDataSerializers":
            raise ExtractionError(
                f"{source.relative_path}:{call.span.start_point[0] + 1}: "
                f"unsupported serializer reference {serializer_expression!r}"
            )
        enclosing = enclosing_type(source.types, call.node)
        if enclosing is None:
            raise ExtractionError(f"{source.relative_path}: defineId outside a type")
        field = _field_for_call(source, call)
        accessors.append(
            Accessor(
                _resolve_owner(
                    owner_expression[: -len(".class")], enclosing, known_types
                ),
                field.name,
                serializer_name,
                -1,
                source.relative_path,
                field.span,
            )
        )
    return accessors


def attach_defaults(
    source: JavaSource,
    accessors: dict[tuple[str, str], Accessor],
) -> int:
    attached = 0
    for call in source.calls:
        if call.name != "define" or len(call.arguments) != 2 or call.owner is None:
            continue
        accessor_name = call.arguments[0].canonical.replace(" ", "").rsplit(".", 1)[-1]
        accessor = accessors.get((call.owner, accessor_name))
        if accessor is None:
            continue
        if accessor.default_expression is not None:
            raise ExtractionError(
                f"{source.relative_path}:{call.span.start_point[0] + 1}: "
                f"duplicate default for {call.owner}.{accessor_name}"
            )
        accessor.default_expression = call.arguments[1].canonical
        accessor.default_span = call.arguments[1].span
        accessor.default_node = call.arguments[1].node
        attached += 1
    return attached


def _literal_integer(text: str) -> int:
    value = text.replace("_", "").rstrip("lL")
    if value.lower().startswith("0x"):
        return int(value, 16)
    if value.lower().startswith("0b"):
        return int(value, 2)
    if len(value) > 1 and value.startswith("0"):
        return int(value, 8)
    return int(value, 10)


def _operator(source: JavaSource, node: Node) -> str | None:
    for child in node.children:
        if not child.is_named:
            value = source.text(child)
            if value not in {"(", ")"}:
                return value
    return None


def evaluate_integer_expression(
    source: JavaSource,
    node: Node,
    constants: dict[str, int],
) -> int:
    """Evaluate the narrow constant-expression subset used by metadata flags."""

    if node.type in {
        "decimal_integer_literal",
        "hex_integer_literal",
        "binary_integer_literal",
        "octal_integer_literal",
    }:
        return _literal_integer(source.text(node))
    if node.type == "identifier":
        name = source.text(node)
        if name in constants:
            return constants[name]
        raise ExtractionError(f"unresolved integer constant {name}")
    if node.type in {"parenthesized_expression", "cast_expression"}:
        value_node = node.child_by_field_name("value")
        if value_node is None:
            named = node.named_children
            value_node = named[-1] if named else None
        if value_node is None:
            raise ExtractionError("integer expression wrapper lacks a value")
        return evaluate_integer_expression(source, value_node, constants)
    if node.type == "unary_expression":
        value_node = node.child_by_field_name("operand")
        if value_node is None:
            value_node = node.named_children[-1]
        value = evaluate_integer_expression(source, value_node, constants)
        operator = _operator(source, node)
        if operator == "+":
            return value
        if operator == "-":
            return -value
        if operator == "~":
            return ~value
    if node.type == "binary_expression":
        left = node.child_by_field_name("left")
        right = node.child_by_field_name("right")
        if left is None or right is None:
            raise ExtractionError("binary integer expression lacks an operand")
        lhs = evaluate_integer_expression(source, left, constants)
        rhs = evaluate_integer_expression(source, right, constants)
        operator = _operator(source, node)
        if operator == "|":
            return lhs | rhs
        if operator == "&":
            return lhs & rhs
        if operator == "^":
            return lhs ^ rhs
        if operator == "<<":
            return lhs << rhs
        if operator == ">>":
            return lhs >> rhs
        if operator == ">>>":
            return (lhs & 0xFFFFFFFF) >> rhs
    raise ExtractionError(
        f"unsupported integer expression node {node.type}: "
        f"{canonical_expression(source.source, node)}"
    )


def _identifier_names(source: JavaSource, node: Node) -> set[str]:
    return {
        source.text(item)
        for item in descendants(node)
        if item.type == "identifier"
    }


def extract_flags(
    source: JavaSource,
    accessors: Sequence[Accessor],
) -> list[Flag]:
    own_accessors: dict[str, set[str]] = {}
    for accessor in accessors:
        if accessor.source_path == source.relative_path:
            own_accessors.setdefault(accessor.owner, set()).add(accessor.java_name)
    flags: list[Flag] = []
    for owner, accessor_names in own_accessors.items():
        fields = source.fields_for(owner)
        integer_fields = [
            item
            for item in fields
            if item.initializer is not None
            and "static" in item.modifiers
            and "final" in item.modifiers
            and item.type_spelling.replace(" ", "") in {"byte", "int", "long", "short"}
        ]
        resolved: dict[str, int] = {}
        pending = {item.name: item for item in integer_fields}
        changed = True
        while changed:
            changed = False
            for name, item in list(pending.items()):
                try:
                    value = evaluate_integer_expression(
                        source, item.initializer.node, resolved
                    )
                except (ExtractionError, ValueError):
                    continue
                resolved[name] = value
                del pending[name]
                changed = True

        methods = source.methods_for(owner)
        method_names: dict[str, set[str]] = {}
        helper_uses_bit_index: dict[str, bool] = {}
        for method in methods:
            body = method.node.child_by_field_name("body")
            if body is None:
                continue
            used = _identifier_names(source, body) & accessor_names
            method_names.setdefault(method.name, set()).update(used)
            helper_uses_bit_index[method.name] = any(
                node.type == "binary_expression"
                and "<<"
                in [source.text(child) for child in node.children if not child.is_named]
                and any(
                    child.type == "decimal_integer_literal"
                    and source.text(child) == "1"
                    for child in node.named_children
                )
                for node in descendants(body)
            )

        mask_bindings: dict[int, set[str]] = {}
        bit_bindings: dict[int, set[str]] = {}
        for method in methods:
            body = method.node.child_by_field_name("body")
            if body is None:
                continue
            for node in descendants(body):
                if node.type == "binary_expression":
                    operators = [
                        source.text(child)
                        for child in node.children
                        if not child.is_named
                    ]
                    if "&" not in operators:
                        continue
                    node_accessors = _identifier_names(source, node) & accessor_names
                    if len(node_accessors) != 1:
                        continue
                    for candidate in (
                        node.child_by_field_name("left"),
                        node.child_by_field_name("right"),
                    ):
                        if candidate is None:
                            continue
                        try:
                            value = evaluate_integer_expression(
                                source, candidate, resolved
                            )
                        except (ExtractionError, ValueError):
                            continue
                        mask_bindings.setdefault(value, set()).update(node_accessors)
                elif node.type == "method_invocation":
                    name_node = node.child_by_field_name("name")
                    arguments = node.child_by_field_name("arguments")
                    if name_node is None or arguments is None:
                        continue
                    helper_name = source.text(name_node)
                    helper_accessors = method_names.get(helper_name, set())
                    if len(helper_accessors) != 1:
                        continue
                    for argument in arguments.named_children:
                        try:
                            value = evaluate_integer_expression(
                                source, argument, resolved
                            )
                        except (ExtractionError, ValueError):
                            continue
                        target = (
                            bit_bindings
                            if helper_uses_bit_index.get(helper_name, False)
                            else mask_bindings
                        )
                        target.setdefault(value, set()).update(helper_accessors)

        for item in integer_fields:
            if "FLAG" not in item.name:
                continue
            raw_value = resolved.get(item.name)
            if raw_value in bit_bindings:
                candidates = set(bit_bindings[raw_value])
                value = 1 << raw_value
            else:
                if raw_value in {None, 0}:
                    continue
                candidates = set(mask_bindings.get(raw_value, set()))
                value = raw_value
            if len(candidates) == 1:
                accessor_name = next(iter(candidates))
                status = "resolved"
                reason = None
            elif not candidates:
                accessor_name = None
                status = "ambiguous"
                reason = "no accessor binding found"
            else:
                accessor_name = None
                status = "ambiguous"
                reason = "multiple accessor bindings: " + ", ".join(sorted(candidates))
            flags.append(
                Flag(
                    owner,
                    item.name,
                    item.initializer.canonical,
                    value,
                    accessor_name,
                    status,
                    reason,
                    source.relative_path,
                    item.initializer.span,
                )
            )
    return flags


def serializer_registration_names(source: JavaSource) -> list[str]:
    names: list[str] = []
    for call in source.calls:
        if call.name != "registerSerializer" or len(call.arguments) != 1:
            continue
        name = call.arguments[0].canonical.replace(" ", "").rsplit(".", 1)[-1]
        if name != "serializer":
            names.append(name)
    if not names or len(names) != len(set(names)):
        raise ExtractionError("serializer registration block is missing or duplicated")
    return names


def _generic_type_argument(source: JavaSource, field: JavaField) -> str | None:
    declaration = field.node.parent
    if declaration is None:
        return None
    type_node = declaration.child_by_field_name("type")
    if type_node is None or type_node.type != "generic_type":
        return None
    arguments = next(
        (child for child in type_node.named_children if child.type == "type_arguments"),
        None,
    )
    if arguments is None or len(arguments.named_children) != 1:
        return None
    return canonical_expression(source.source, arguments.named_children[0]).replace(
        " ", ""
    )


def parse_entity_types(
    source: JavaSource,
    known_types: dict[str, JavaType],
) -> list[EntityTypeRegistration]:
    owner = known_types.get("net.minecraft.world.entity.EntityType")
    if owner is None:
        raise ExtractionError("EntityType.java did not declare EntityType")
    registrations: list[EntityTypeRegistration] = []
    for field in source.fields_for(owner):
        runtime_spelling = _generic_type_argument(source, field)
        if runtime_spelling is None or field.initializer is None:
            continue
        invocation = field.initializer.node
        if invocation.type != "method_invocation":
            continue
        name_node = invocation.child_by_field_name("name")
        arguments = invocation.child_by_field_name("arguments")
        if name_node is None or source.text(name_node) != "register" or arguments is None:
            continue
        values = arguments.named_children
        if len(values) < 2 or values[0].type != "string_literal":
            raise ExtractionError(
                f"{source.relative_path}:{field.span.start_point[0] + 1}: "
                "EntityType register requires a literal resource id and builder"
            )
        resource_path = source.text(values[0])[1:-1]
        runtime_class = resolve_type_reference(runtime_spelling, owner, known_types)
        diagnostic = (
            None
            if runtime_class is not None
            else f"unresolved runtime class {runtime_spelling}"
        )
        registrations.append(
            EntityTypeRegistration(
                len(registrations),
                field.name,
                "minecraft:" + resource_path,
                runtime_class,
                diagnostic,
                field.span,
            )
        )
    if len(registrations) != 157:
        raise ExtractionError(
            f"expected 157 EntityType registrations, parsed {len(registrations)}"
        )
    return registrations


def _ancestor_accessor_count(
    owner: str,
    classes: dict[str, JavaType],
    accessors: dict[str, list[Accessor]],
    visiting: set[str] | None = None,
) -> int:
    visiting = set() if visiting is None else set(visiting)
    if owner in visiting:
        raise ExtractionError(f"inheritance cycle involving {owner}")
    visiting.add(owner)
    item = classes.get(owner)
    if item is None or item.superclass is None:
        return 0
    return _ancestor_accessor_count(
        item.superclass, classes, accessors, visiting
    ) + len(accessors.get(item.superclass, []))


def assign_data_ids(
    classes: Sequence[JavaType], accessors: Sequence[Accessor]
) -> None:
    class_map = {item.fqcn: item for item in classes}
    by_owner: dict[str, list[Accessor]] = {}
    for accessor in accessors:
        by_owner.setdefault(accessor.owner, []).append(accessor)
    for owner, items in by_owner.items():
        items.sort(key=lambda item: item.declaration_span.start_byte)
        inherited = _ancestor_accessor_count(owner, class_map, by_owner)
        for order, accessor in enumerate(items):
            accessor.declaration_order = order
            accessor.data_id = inherited + order


def _manifest_digest(files: Iterable[tuple[str, bytes]]) -> str:
    digest = hashlib.sha256()
    for relative_path, source in sorted(files):
        digest.update(relative_path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(hashlib.sha256(source).digest())
        digest.update(b"\n")
    return digest.hexdigest()


def _source_for_owner(
    sources: dict[str, JavaSource], owner: str
) -> JavaSource:
    matches = [
        source
        for source in sources.values()
        if any(java_type.fqcn == owner for java_type in source.types)
    ]
    if len(matches) != 1:
        raise ExtractionError(
            f"{owner}: expected one source owner, found {len(matches)}"
        )
    return matches[0]


def _method_invocation_parts(
    source: JavaSource, node: Node
) -> tuple[str, Node | None, list[Node]]:
    if node.type != "method_invocation":
        raise ExtractionError(
            f"{source.relative_path}:{node.start_point.row + 1}: "
            f"expected method invocation, found {node.type}"
        )
    name = node.child_by_field_name("name")
    arguments = node.child_by_field_name("arguments")
    if name is None or arguments is None:
        raise ExtractionError(
            f"{source.relative_path}:{node.start_point.row + 1}: "
            "incomplete method invocation"
        )
    return (
        source.text(name),
        node.child_by_field_name("object"),
        list(arguments.named_children),
    )


def _qualified_factory_owner(
    sources: dict[str, JavaSource],
    source: JavaSource,
    current_owner: str,
    qualifier: Node | None,
    method_name: str,
    known_types: dict[str, JavaType],
) -> str:
    candidate = current_owner
    if qualifier is not None:
        qualifier_text = source.expression(qualifier).canonical.replace(" ", "")
        current_type = known_types[current_owner]
        candidate = resolve_type_reference(qualifier_text, current_type, known_types)
        if candidate is None:
            raise ExtractionError(
                f"{current_owner}.{method_name}: unresolved factory qualifier "
                f"{qualifier_text!r}"
            )

    seen: set[str] = set()
    while candidate is not None:
        if candidate in seen:
            raise ExtractionError(
                f"{current_owner}.{method_name}: attribute factory inheritance cycle"
            )
        seen.add(candidate)
        candidate_source = _source_for_owner(sources, candidate)
        if any(
            method.name == method_name and len(method.parameters) == 0
            for method in candidate_source.methods_for(candidate)
        ):
            return candidate
        java_type = known_types.get(candidate)
        candidate = java_type.superclass if java_type is not None else None
    raise ExtractionError(
        f"{current_owner}.{method_name}: no zero-argument attribute factory in hierarchy"
    )


def _double_literal_bits(source: JavaSource, node: Node, context: str) -> int:
    expression = source.expression(node)
    literal = expression.canonical.replace("_", "").replace(" ", "")
    suffix = literal[-1:] if literal[-1:] in {"f", "F", "d", "D"} else ""
    text = literal[:-1] if suffix else literal
    try:
        value = float(text)
    except ValueError as error:
        raise ExtractionError(f"{context}: expected numeric literal {literal!r}") from error
    if suffix in {"f", "F"}:
        value = struct.unpack("!f", struct.pack("!f", value))[0]
    if not math.isfinite(value):
        raise ExtractionError(f"{context}: numeric literal must be finite")
    return struct.unpack("!Q", struct.pack("!d", value))[0]


def _attribute_factory_defaults(
    sources: dict[str, JavaSource],
    known_types: dict[str, JavaType],
    owner: str,
    method_name: str,
    attribute_names: set[str],
    cache: dict[tuple[str, str], tuple[tuple[str, int | None], ...]],
    active: set[tuple[str, str]],
) -> tuple[tuple[str, int | None], ...]:
    key = (owner, method_name)
    if key in cache:
        return cache[key]
    if key in active:
        raise ExtractionError(
            f"{owner}.{method_name}: recursive attribute supplier factory"
        )
    active.add(key)
    source = _source_for_owner(sources, owner)
    methods = [
        method
        for method in source.methods_for(owner)
        if method.name == method_name and len(method.parameters) == 0
    ]
    if len(methods) != 1:
        raise ExtractionError(
            f"{owner}.{method_name}: expected one zero-argument factory, found {len(methods)}"
        )
    body = methods[0].node.child_by_field_name("body")
    returns = [
        node for node in descendants(body) if node.type == "return_statement"
    ]
    if len(returns) != 1 or len(returns[0].named_children) != 1:
        raise ExtractionError(
            f"{owner}.{method_name}: expected one direct return expression"
        )

    node = returns[0].named_children[0]
    own_defaults: list[tuple[str, int | None]] = []
    while True:
        call_name, qualifier, arguments = _method_invocation_parts(source, node)
        if call_name != "add":
            break
        if len(arguments) not in {1, 2}:
            raise ExtractionError(
                f"{owner}.{method_name}: AttributeSupplier.Builder.add has "
                f"{len(arguments)} arguments"
            )
        attribute = source.expression(arguments[0]).canonical.replace(" ", "")
        if not attribute.startswith("Attributes."):
            raise ExtractionError(
                f"{owner}.{method_name}: unsupported attribute expression {attribute!r}"
            )
        constant = attribute.removeprefix("Attributes.")
        if constant not in attribute_names:
            raise ExtractionError(
                f"{owner}.{method_name}: unknown Attributes constant {constant}"
            )
        override = (
            _double_literal_bits(
                source,
                arguments[1],
                f"{owner}.{method_name} Attributes.{constant}",
            )
            if len(arguments) == 2
            else None
        )
        own_defaults.append((constant, override))
        if qualifier is None:
            raise ExtractionError(
                f"{owner}.{method_name}: add call lacks a builder qualifier"
            )
        node = qualifier

    call_name, qualifier, arguments = _method_invocation_parts(source, node)
    if arguments:
        raise ExtractionError(
            f"{owner}.{method_name}: base factory {call_name} takes arguments"
        )
    qualifier_text = (
        source.expression(qualifier).canonical.replace(" ", "")
        if qualifier is not None
        else None
    )
    if call_name == "builder" and qualifier_text == "AttributeSupplier":
        inherited: tuple[tuple[str, int | None], ...] = ()
    else:
        factory_owner = _qualified_factory_owner(
            sources, source, owner, qualifier, call_name, known_types
        )
        if factory_owner == ATTRIBUTE_SUPPLIER_OWNER:
            raise ExtractionError(
                f"{owner}.{method_name}: unsupported AttributeSupplier factory {call_name}"
            )
        inherited = _attribute_factory_defaults(
            sources,
            known_types,
            factory_owner,
            call_name,
            attribute_names,
            cache,
            active,
        )

    ordered = list(inherited)
    positions = {constant: index for index, (constant, _) in enumerate(ordered)}
    for constant, override in reversed(own_defaults):
        if constant in positions:
            ordered[positions[constant]] = (constant, override)
        else:
            positions[constant] = len(ordered)
            ordered.append((constant, override))
    result = tuple(ordered)
    if not result:
        raise ExtractionError(f"{owner}.{method_name}: empty attribute supplier")
    cache[key] = result
    active.remove(key)
    return result


def extract_attribute_suppliers(
    sources: dict[str, JavaSource],
    classes: list[JavaType],
    entity_types: list[EntityTypeRegistration],
) -> list[EntityTypeRegistration]:
    """Attach each living entity's exact Vanilla `DefaultAttributes` membership."""

    known_types = {item.fqcn: item for item in classes}
    default_source = _source_for_owner(sources, DEFAULT_ATTRIBUTES_OWNER)
    default_type = known_types[DEFAULT_ATTRIBUTES_OWNER]
    supplier_fields = [
        field
        for field in default_source.fields_for(DEFAULT_ATTRIBUTES_OWNER)
        if field.name == "SUPPLIERS"
    ]
    if len(supplier_fields) != 1 or supplier_fields[0].initializer is None:
        raise ExtractionError(
            "DefaultAttributes.SUPPLIERS: expected one initialized field"
        )
    supplier_span = supplier_fields[0].initializer.span
    factories: dict[str, tuple[str, str]] = {}
    for call in default_source.calls_for(DEFAULT_ATTRIBUTES_OWNER):
        if (
            call.name != "put"
            or call.span.start_byte < supplier_span.start_byte
            or call.span.end_byte > supplier_span.end_byte
        ):
            continue
        if len(call.arguments) != 2:
            raise ExtractionError(
                "DefaultAttributes.SUPPLIERS: put requires entity and supplier"
            )
        entity = call.arguments[0].node
        entity_owner = entity.child_by_field_name("object")
        entity_field = entity.child_by_field_name("field")
        if (
            entity.type != "field_access"
            or entity_owner is None
            or default_source.text(entity_owner) != "EntityType"
            or entity_field is None
        ):
            raise ExtractionError(
                "DefaultAttributes.SUPPLIERS: unsupported entity key "
                + call.arguments[0].canonical
            )
        constant_name = default_source.text(entity_field)
        build_name, provider, build_arguments = _method_invocation_parts(
            default_source, call.arguments[1].node
        )
        if build_name != "build" or provider is None or build_arguments:
            raise ExtractionError(
                f"DefaultAttributes.{constant_name}: expected factory().build()"
            )
        factory_name, factory_qualifier, factory_arguments = _method_invocation_parts(
            default_source, provider
        )
        if factory_qualifier is None or factory_arguments:
            raise ExtractionError(
                f"DefaultAttributes.{constant_name}: unsupported supplier factory"
            )
        factory_owner = _qualified_factory_owner(
            sources,
            default_source,
            default_type.fqcn,
            factory_qualifier,
            factory_name,
            known_types,
        )
        if constant_name in factories:
            raise ExtractionError(
                f"DefaultAttributes.SUPPLIERS: duplicate EntityType.{constant_name}"
            )
        factories[constant_name] = (factory_owner, factory_name)

    attribute_source = _source_for_owner(sources, ATTRIBUTES_OWNER)
    attribute_names = {
        field.name for field in attribute_source.fields_for(ATTRIBUTES_OWNER)
    }
    cache: dict[tuple[str, str], tuple[tuple[str, int | None], ...]] = {}
    living_owner = "net.minecraft.world.entity.LivingEntity"
    output: list[EntityTypeRegistration] = []
    for entity_type in entity_types:
        current = entity_type.runtime_class
        is_living = False
        while current is not None:
            if current == living_owner:
                is_living = True
                break
            java_type = known_types.get(current)
            current = java_type.superclass if java_type is not None else None
        factory = factories.get(entity_type.constant_name)
        if is_living != (factory is not None):
            raise ExtractionError(
                f"EntityType.{entity_type.constant_name}: living={is_living} "
                f"but DefaultAttributes supplier={factory is not None}"
            )
        if factory is None:
            output.append(entity_type)
            continue
        factory_owner, factory_method = factory
        defaults = _attribute_factory_defaults(
            sources,
            known_types,
            factory_owner,
            factory_method,
            attribute_names,
            cache,
            set(),
        )
        output.append(
            replace(
                entity_type,
                attribute_factory_owner=factory_owner,
                attribute_factory_method=factory_method,
                attribute_constants=tuple(constant for constant, _ in defaults),
                attribute_base_overrides=defaults,
            )
        )
    if set(factories) != {
        item.constant_name for item in output if item.attribute_constants
    }:
        raise ExtractionError(
            "DefaultAttributes.SUPPLIERS does not match generated living entities"
        )
    return output


def extract(source_root: Path) -> ExtractionResult:
    entity_root = source_root / ENTITY_TREE
    if not entity_root.is_dir():
        raise ExtractionError(
            f"missing vanilla entity source tree: {entity_root}"
        )
    entity_paths = sorted(
        path.relative_to(source_root).as_posix()
        for path in entity_root.rglob("*.java")
    )
    all_paths = sorted(
        set(
            entity_paths
            + [path.as_posix() for path in SUPPORTING_SOURCES]
        )
    )
    source_bytes = {
        relative: (source_root / relative).read_bytes() for relative in all_paths
    }
    sources = {
        relative: parse_source(relative, contents)
        for relative, contents in source_bytes.items()
    }
    classes = [
        item
        for relative in entity_paths
        for item in sources[relative].types
    ]
    resolve_class_hierarchy(classes)
    known_types = {item.fqcn: item for item in classes}

    accessors = [
        item
        for relative in entity_paths
        for item in extract_accessors(sources[relative], known_types)
    ]
    keys = [(item.owner, item.java_name) for item in accessors]
    if len(keys) != len(set(keys)):
        raise ExtractionError("duplicate accessor owner/name")
    if len(accessors) != 227:
        raise ExtractionError(
            f"expected 227 Tree-sitter defineId calls, extracted {len(accessors)}"
        )
    assign_data_ids(classes, accessors)

    accessor_map = {(item.owner, item.java_name): item for item in accessors}
    attached_defaults = sum(
        attach_defaults(sources[relative], accessor_map)
        for relative in entity_paths
    )
    missing = [
        f"{item.owner}.{item.java_name}"
        for item in accessors
        if item.default_expression is None
    ]
    if missing:
        raise ExtractionError("accessors missing defaults: " + ", ".join(missing))
    if attached_defaults != 227:
        raise ExtractionError(
            f"expected 227 Tree-sitter defaults, attached {attached_defaults}"
        )

    serializers = serializer_registration_names(
        sources[SERIALIZER_SOURCE.as_posix()]
    )
    unknown_serializers = sorted(
        {item.serializer for item in accessors} - set(serializers)
    )
    if unknown_serializers:
        raise ExtractionError(
            "unknown entity-data serializers: "
            + ", ".join(unknown_serializers)
        )
    flags = [
        item
        for relative in entity_paths
        for item in extract_flags(sources[relative], accessors)
    ]
    entity_types = parse_entity_types(
        sources[ENTITY_TYPE_SOURCE.as_posix()], known_types
    )
    entity_types = extract_attribute_suppliers(sources, classes, entity_types)
    return ExtractionResult(
        sorted(classes, key=lambda item: item.fqcn),
        sorted(accessors, key=lambda item: (item.owner, item.declaration_order)),
        sorted(flags, key=lambda item: (item.owner, item.java_name)),
        entity_types,
        serializers,
        _manifest_digest(source_bytes.items()),
        sources,
    )
