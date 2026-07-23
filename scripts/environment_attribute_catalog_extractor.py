"""Tree-sitter-backed extraction of Vanilla environment-attribute declarations."""

from __future__ import annotations

import ast
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
