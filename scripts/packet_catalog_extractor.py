"""Tree-sitter-backed domain extractor for vanilla packet catalogs.

Java lexing and structure come exclusively from :mod:`vanilla_java_source`.
This module only recognizes Minecraft's PacketType declarations and protocol
builder registrations; Rust rendering remains in generate-packet-catalog.py.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Iterable

from vanilla_java_source import JavaExpression, JavaSource


class PacketCatalogExtractionError(ValueError):
    """Explicit unsupported or inconsistent packet catalog source."""


@dataclass(frozen=True)
class Packet:
    constant: str
    resource_id: str
    class_name: str


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise PacketCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def _string_literal(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "packet resource id is not a string literal")
    try:
        value = json.loads(source.text(expression.node))
    except (json.JSONDecodeError, TypeError) as error:
        _fail(source, expression, f"invalid packet resource string: {error}")
    if not isinstance(value, str) or not value:
        _fail(source, expression, "packet resource id is empty or non-string")
    return value


def packet_resource_ids(sources: Iterable[JavaSource]) -> dict[str, str]:
    resources: dict[str, str] = {}
    found = 0
    for source in sources:
        for field in source.fields:
            if not field.type_spelling.replace(" ", "").startswith("PacketType<"):
                continue
            found += 1
            initializer = field.initializer
            if initializer is None or initializer.node.type != "method_invocation":
                expression = initializer or JavaExpression("", field.span, field.node)
                _fail(source, expression, f"PacketType field {field.name} lacks a factory call")
            name = initializer.node.child_by_field_name("name")
            arguments = initializer.node.child_by_field_name("arguments")
            if name is None or arguments is None:
                _fail(source, initializer, f"PacketType field {field.name} has an incomplete call")
            factory = source.text(name)
            if factory not in {"createClientbound", "createServerbound"}:
                _fail(
                    source,
                    initializer,
                    f"PacketType field {field.name} uses unsupported factory {factory}",
                )
            if len(arguments.named_children) != 1:
                _fail(
                    source,
                    initializer,
                    f"PacketType field {field.name} factory has "
                    f"{len(arguments.named_children)} arguments",
                )
            resource_id = "minecraft:" + _string_literal(
                source, source.expression(arguments.named_children[0])
            )
            previous = resources.setdefault(field.name, resource_id)
            if previous != resource_id:
                _fail(
                    source,
                    initializer,
                    f"conflicting resource ids for {field.name}: {previous}, {resource_id}",
                )
    if found == 0:
        raise PacketCatalogExtractionError("no PacketType fields found")
    return resources


def _qualified_member(
    source: JavaSource, expression: JavaExpression, description: str
) -> tuple[str, str]:
    node = expression.node
    if node.type not in {"field_access", "scoped_identifier"}:
        _fail(source, expression, f"{description} is not a qualified member")
    object_node = node.child_by_field_name("object") or node.child_by_field_name("scope")
    field_node = node.child_by_field_name("field") or node.child_by_field_name("name")
    if object_node is None or field_node is None:
        _fail(source, expression, f"{description} is incomplete")
    return source.expression(object_node).canonical.replace(" ", ""), source.text(field_node)


def _packet_type(source: JavaSource, expression: JavaExpression) -> str:
    _owner, constant = _qualified_member(source, expression, "packet type")
    return constant


def _codec_class(source: JavaSource, expression: JavaExpression) -> str:
    owner, codec = _qualified_member(source, expression, "packet codec")
    if not codec.endswith("CODEC"):
        _fail(source, expression, f"packet codec member {codec} does not end in CODEC")
    return owner


def _constructed_class(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "object_creation_expression":
        _fail(source, expression, "bundle delimiter is not an object creation expression")
    type_node = expression.node.child_by_field_name("type")
    if type_node is None:
        _fail(source, expression, "bundle delimiter construction lacks a type")
    return source.expression(type_node).canonical.replace(" ", "")


def _packet_from_registration(
    source: JavaSource,
    packet_type: str,
    class_name: str,
    direction: str,
    resources: dict[str, str],
    evidence: JavaExpression,
) -> Packet:
    prefix = direction.upper()
    constant = (
        packet_type
        if packet_type.startswith(("CLIENTBOUND_", "SERVERBOUND_"))
        else f"{prefix}_{packet_type}"
    )
    if not constant.startswith(f"{prefix}_"):
        _fail(source, evidence, f"{packet_type} registered in wrong direction {direction}")
    try:
        resource_id = resources[packet_type]
    except KeyError:
        _fail(source, evidence, f"missing PacketTypes resource id for {packet_type}")
    return Packet(constant, resource_id, class_name)


def registered_packets(
    source: JavaSource, direction: str, resources: dict[str, str]
) -> list[Packet]:
    if direction not in {"serverbound", "clientbound"}:
        raise PacketCatalogExtractionError(f"unsupported packet direction {direction}")
    marker = f"{direction.upper()}_TEMPLATE"
    fields = [field for field in source.fields if field.name == marker]
    if len(fields) != 1 or fields[0].initializer is None:
        raise PacketCatalogExtractionError(
            f"{source.relative_path}: expected one initialized {marker}, found {len(fields)}"
        )
    initializer = fields[0].initializer
    calls = [
        call
        for call in source.calls
        if initializer.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= initializer.span.end_byte
        and call.name in {"addPacket", "withBundlePacket"}
    ]
    calls.sort(key=lambda call: (call.span.end_byte, call.span.start_byte))
    packets: list[Packet] = []
    bundle_count = 0
    for call in calls:
        evidence = source.expression(call.node)
        if call.name == "addPacket":
            if len(call.arguments) not in {2, 3}:
                _fail(source, evidence, f"addPacket has {len(call.arguments)} arguments")
            packet_type = _packet_type(source, call.arguments[0])
            class_name = _codec_class(source, call.arguments[1])
            if len(call.arguments) == 3 and call.arguments[2].node.type not in {
                "identifier",
                "field_access",
                "scoped_identifier",
            }:
                _fail(source, call.arguments[2], "packet codec modifier is not a field reference")
        else:
            bundle_count += 1
            if bundle_count != 1 or packets:
                _fail(source, evidence, "withBundlePacket must be the unique first registration")
            if len(call.arguments) != 3:
                _fail(source, evidence, f"withBundlePacket has {len(call.arguments)} arguments")
            packet_type = _packet_type(source, call.arguments[0]) + "_DELIMITER"
            class_name = _constructed_class(source, call.arguments[2])
        packets.append(
            _packet_from_registration(
                source, packet_type, class_name, direction, resources, evidence
            )
        )
    if not packets:
        raise PacketCatalogExtractionError(
            f"{source.relative_path}: no {direction} packet registrations found in {marker}"
        )
    constants = [packet.constant for packet in packets]
    if len(constants) != len(set(constants)):
        raise PacketCatalogExtractionError(
            f"{source.relative_path}: duplicate {direction} packet registration"
        )
    return packets
