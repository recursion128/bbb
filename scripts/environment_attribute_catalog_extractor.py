"""Tree-sitter-backed extraction of Vanilla environment-attribute declarations."""

from __future__ import annotations

import ast
import re
import struct
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaField, JavaSource


class EnvironmentAttributeExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class EnvironmentAttributeEntry:
    constant_name: str
    resource_id: str
    value_type: str
    value_range: str
    syncable: bool
    positional: bool
    spatially_interpolated: bool
    scalar_default: bool | int | None
    color_default: int | None


@dataclass(frozen=True)
class EnvironmentAttributeTypeEntry:
    constant_name: str
    resource_id: str
    modifier_family: str


ATTRIBUTE_TYPES = {
    "AMBIENT_PARTICLES",
    "AMBIENT_SOUNDS",
    "ANGLE_DEGREES",
    "ACTIVITY",
    "ARGB_COLOR",
    "BACKGROUND_MUSIC",
    "BED_RULE",
    "BOOLEAN",
    "FLOAT",
    "INTEGER",
    "MOON_PHASE",
    "PARTICLE",
    "RGB_COLOR",
    "TRI_STATE",
}

VALUE_RANGES = {
    "AttributeRange.NON_NEGATIVE_FLOAT": "NON_NEGATIVE_FLOAT",
    "AttributeRange.UNIT_FLOAT": "UNIT_FLOAT",
    "AttributeRange.ofFloat(0.0F,15.0F)": "SKY_LIGHT",
}

MODIFIER_FAMILIES = {
    "BOOLEAN",
    "FLOAT",
    "RGB",
    "ARGB",
    "INTEGER",
    "OVERRIDE_ONLY",
}


def _fail(
    source: JavaSource,
    expression: JavaExpression,
    message: str,
) -> None:
    line, column = expression.span.start_point
    raise EnvironmentAttributeExtractionError(
        f"{source.relative_path}:{line + 1}:{column + 1}: {message}"
    )


def _calls(source: JavaSource, expression: JavaExpression, name: str) -> list:
    return [
        call
        for call in source.calls
        if call.name == name
        and expression.span.start_byte <= call.span.start_byte
        and call.span.end_byte <= expression.span.end_byte
    ]


def _string(source: JavaSource, expression: JavaExpression) -> str:
    if expression.node.type != "string_literal":
        _fail(source, expression, "expected string literal")
    try:
        value = ast.literal_eval(source.text(expression.node))
    except (SyntaxError, ValueError) as error:
        _fail(source, expression, f"invalid Java string literal: {error}")
    if not isinstance(value, str):
        _fail(source, expression, "expected string literal")
    return value


def _register_arguments(
    source: JavaSource,
    field: JavaField,
) -> tuple[JavaExpression, ...]:
    initializer = field.initializer
    if initializer is None or initializer.node.type != "method_invocation":
        expression = initializer or JavaExpression("", field.span, field.node)
        _fail(source, expression, f"{field.name} lacks a register call")
    name = initializer.node.child_by_field_name("name")
    arguments = initializer.node.child_by_field_name("arguments")
    if name is None or arguments is None or source.text(name) != "register":
        _fail(source, initializer, f"{field.name} is not initialized by register")
    return tuple(source.expression(node) for node in arguments.named_children)


def _single_zero_argument_flag(
    source: JavaSource,
    initializer: JavaExpression,
    name: str,
) -> bool:
    calls = _calls(source, initializer, name)
    if len(calls) > 1 or any(call.arguments for call in calls):
        _fail(source, initializer, f"ambiguous {name} builder flag")
    return bool(calls)


def _scalar_default(
    source: JavaSource, initializer: JavaExpression, value_type: str
) -> bool | int | None:
    if value_type not in {"BOOLEAN", "FLOAT", "ANGLE_DEGREES"}:
        return None
    calls = _calls(source, initializer, "defaultValue")
    if len(calls) != 1 or len(calls[0].arguments) != 1:
        _fail(source, initializer, "expected one scalar defaultValue")
    expression = calls[0].arguments[0]
    spelling = expression.canonical.replace(" ", "")
    if value_type == "BOOLEAN":
        if spelling not in {"true", "false"}:
            _fail(source, expression, "expected boolean default literal")
        return spelling == "true"
    node = expression.node
    if node.type == "unary_expression":
        operator = node.child_by_field_name("operator")
        operand = node.child_by_field_name("operand")
        if operator is None or operand is None or source.text(operator) != "-":
            _fail(source, expression, "expected float default literal")
        node = operand
    if node.type != "decimal_floating_point_literal" or not spelling.lower().endswith("f"):
        _fail(source, expression, "expected float default literal")
    try:
        return struct.unpack(">I", struct.pack(">f", float(spelling[:-1].replace("_", ""))))[0]
    except (ValueError, OverflowError) as error:
        _fail(source, expression, f"invalid float default literal: {error}")


def _color_default(source: JavaSource, initializer: JavaExpression, value_type: str) -> int | None:
    if value_type not in {"RGB_COLOR", "ARGB_COLOR"}:
        return None
    calls = _calls(source, initializer, "defaultValue")
    if len(calls) != 1 or len(calls[0].arguments) != 1:
        _fail(source, initializer, "expected one color defaultValue")
    expression = calls[0].arguments[0]
    node = expression.node
    if node.type == "unary_expression":
        operator = node.child_by_field_name("operator")
        operand = node.child_by_field_name("operand")
        if operator is None or operand is None or source.text(operator) != "-":
            _fail(source, expression, "expected integer color default literal")
        node = operand
    if node.type != "decimal_integer_literal":
        _fail(source, expression, "expected integer color default literal")
    return int(expression.canonical.replace(" ", "").replace("_", "")) & 0xFFFFFFFF


def environment_attribute_entries(
    source: JavaSource,
    expected_count: int = 48,
) -> list[EnvironmentAttributeEntry]:
    entries: list[EnvironmentAttributeEntry] = []
    for field in source.fields:
        if not field.type_spelling.replace(" ", "").startswith(
            "EnvironmentAttribute<"
        ):
            continue
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(
                source,
                field.initializer,
                f"{field.name} register must have two arguments",
            )
        builder_calls = _calls(source, arguments[1], "builder")
        if len(builder_calls) != 1 or len(builder_calls[0].arguments) != 1:
            _fail(source, arguments[1], f"{field.name} has ambiguous builder type")
        builder_type = builder_calls[0].arguments[0].canonical
        if not builder_type.startswith("AttributeTypes."):
            _fail(
                source,
                builder_calls[0].arguments[0],
                f"{field.name} builder type must reference AttributeTypes",
            )
        value_type = builder_type.removeprefix("AttributeTypes.")
        if value_type not in ATTRIBUTE_TYPES:
            _fail(
                source,
                builder_calls[0].arguments[0],
                f"{field.name} uses unknown attribute type {value_type}",
            )
        range_calls = _calls(source, arguments[1], "valueRange")
        if len(range_calls) > 1 or any(len(call.arguments) != 1 for call in range_calls):
            _fail(source, arguments[1], f"{field.name} has ambiguous value range")
        value_range = "ANY"
        if range_calls:
            range_expression = range_calls[0].arguments[0].canonical.replace(" ", "")
            value_range = VALUE_RANGES.get(range_expression, "")
            if not value_range:
                _fail(
                    source,
                    range_calls[0].arguments[0],
                    f"{field.name} uses unknown value range {range_expression}",
                )
        entries.append(
            EnvironmentAttributeEntry(
                constant_name=field.name,
                resource_id="minecraft:" + _string(source, arguments[0]),
                value_type=value_type,
                value_range=value_range,
                syncable=_single_zero_argument_flag(
                    source, field.initializer, "syncable"
                ),
                positional=not _single_zero_argument_flag(
                    source, field.initializer, "notPositional"
                ),
                spatially_interpolated=_single_zero_argument_flag(
                    source, field.initializer, "spatiallyInterpolated"
                ),
                scalar_default=_scalar_default(source, field.initializer, value_type),
                color_default=_color_default(source, field.initializer, value_type),
            )
        )

    if len(entries) != expected_count:
        raise EnvironmentAttributeExtractionError(
            f"{source.relative_path}: expected {expected_count} environment attributes, "
            f"extracted {len(entries)}"
        )
    for description, values in (
        ("constant", [entry.constant_name for entry in entries]),
        ("resource id", [entry.resource_id for entry in entries]),
    ):
        if len(values) != len(set(values)):
            raise EnvironmentAttributeExtractionError(
                f"{source.relative_path}: duplicate environment attribute {description}"
            )
    return entries


def environment_attribute_type_entries(
    source: JavaSource,
    expected_count: int = 14,
) -> list[EnvironmentAttributeTypeEntry]:
    entries: list[EnvironmentAttributeTypeEntry] = []
    for field in source.fields:
        if not field.type_spelling.replace(" ", "").startswith("AttributeType<"):
            continue
        arguments = _register_arguments(source, field)
        if len(arguments) != 2:
            _fail(
                source,
                field.initializer,
                f"{field.name} register must have two arguments",
            )
        libraries = re.findall(
            r"\bAttributeModifier\.([A-Z_]+)_LIBRARY\b",
            source.text(arguments[1].node),
        )
        if len(libraries) > 1:
            _fail(source, arguments[1], f"{field.name} has ambiguous modifier library")
        modifier_family = "OVERRIDE_ONLY"
        if libraries:
            modifier_family = libraries[0].removesuffix("_COLOR")
        if modifier_family not in MODIFIER_FAMILIES:
            _fail(
                source,
                arguments[1],
                f"{field.name} uses unknown modifier family {modifier_family}",
            )
        entries.append(
            EnvironmentAttributeTypeEntry(
                constant_name=field.name,
                resource_id="minecraft:" + _string(source, arguments[0]),
                modifier_family=modifier_family,
            )
        )

    if len(entries) != expected_count:
        raise EnvironmentAttributeExtractionError(
            f"{source.relative_path}: expected {expected_count} environment attribute types, "
            f"extracted {len(entries)}"
        )
    constants = [entry.constant_name for entry in entries]
    if expected_count == len(ATTRIBUTE_TYPES) and set(constants) != ATTRIBUTE_TYPES:
        missing = sorted(ATTRIBUTE_TYPES - set(constants))
        extra = sorted(set(constants) - ATTRIBUTE_TYPES)
        raise EnvironmentAttributeExtractionError(
            f"{source.relative_path}: environment attribute type inventory changed "
            f"(missing={missing}, extra={extra})"
        )
    for description, values in (
        ("constant", constants),
        ("resource id", [entry.resource_id for entry in entries]),
    ):
        if len(values) != len(set(values)):
            raise EnvironmentAttributeExtractionError(
                f"{source.relative_path}: duplicate environment attribute type {description}"
            )
    return entries
