"""Tree-sitter-backed extraction for command argument type registrations."""

from __future__ import annotations

import json
from dataclasses import dataclass

from vanilla_java_source import JavaExpression, JavaSource


class CommandArgumentCatalogExtractionError(ValueError):
    pass


@dataclass(frozen=True)
class CommandArgumentEntry:
    resource_id: str


def _fail(source: JavaSource, expression: JavaExpression, message: str) -> None:
    span = expression.span
    raise CommandArgumentCatalogExtractionError(
        f"{source.relative_path}:{span.start_point[0] + 1}:"
        f"{span.start_point[1] + 1}: {message}"
    )


def command_argument_entries(
    source: JavaSource, expected_count: int = 57
) -> list[CommandArgumentEntry]:
    methods = [method for method in source.methods if method.name == "bootstrap"]
    if len(methods) != 1 or methods[0].body is None:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected exactly one bootstrap method with a body"
        )
    method = methods[0]
    calls = [
        call
        for call in source.calls
        if call.name == "register"
        and call.qualifier is None
        and method.body.start_byte <= call.span.start_byte
        and call.span.end_byte <= method.body.end_byte
    ]
    calls.sort(key=lambda call: (call.span.start_byte, call.span.end_byte))
    entries: list[CommandArgumentEntry] = []
    for call in calls:
        if len(call.arguments) != 4 or call.arguments[0].canonical != "registry":
            _fail(source, source.expression(call.node), "unsupported argument register call")
        resource = call.arguments[1]
        if resource.node.type != "string_literal":
            _fail(source, resource, "argument id is not a string literal")
        try:
            value = json.loads(source.text(resource.node))
        except (json.JSONDecodeError, TypeError) as error:
            _fail(source, resource, f"invalid argument id literal: {error}")
        if not isinstance(value, str) or not value:
            _fail(source, resource, "argument id is empty or non-string")
        entries.append(CommandArgumentEntry(value if ":" in value else f"minecraft:{value}"))
    if len(entries) != expected_count:
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: expected {expected_count} argument registrations, "
            f"extracted {len(entries)}"
        )
    resources = [entry.resource_id for entry in entries]
    if len(resources) != len(set(resources)):
        raise CommandArgumentCatalogExtractionError(
            f"{source.relative_path}: duplicate command argument resource id"
        )
    return entries
