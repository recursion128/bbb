"""Extract RegistryDataLoader registry lists from vanilla Java CST facts."""

from __future__ import annotations

import ast
import json
import re
from dataclasses import dataclass

from vanilla_java_source import JavaSource, JavaSourceError, expression_fact


@dataclass(frozen=True)
class RegistryDataCatalogEntry:
    constant_name: str
    resource_id: str
    codec: str
    validator: str | None


@dataclass(frozen=True)
class BuiltInRegistryCatalogEntry:
    constant_name: str
    registry_constant_name: str
    resource_id: str
    registration: str
    default_id: str | None


@dataclass(frozen=True)
class StaticRegistryElements:
    resource_id: str
    elements: tuple[str, ...]


def static_registry_elements(
    report: str, expected: list[BuiltInRegistryCatalogEntry]
) -> list[StaticRegistryElements]:
    """Check RegistryDumpReport against the source-derived STATIC identities."""
    def unique_object(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate registry report key {key!r}")
            result[key] = value
        return result

    parsed = json.loads(report, object_pairs_hook=unique_object)
    if not isinstance(parsed, dict) or set(parsed) != {row.resource_id for row in expected}:
        raise ValueError("registry report identities differ from BuiltInRegistries")
    resource_id_pattern = re.compile(r"[a-z0-9_.-]+:[a-z0-9_./-]+")
    result = []
    for registry_id, source in enumerate(expected):
        row = parsed[source.resource_id]
        if not isinstance(row, dict) or set(row) != (
            {"protocol_id", "entries"} | ({"default"} if source.default_id else set())
        ):
            raise ValueError(f"invalid registry report row {source.resource_id}")
        if type(row["protocol_id"]) is not int or row["protocol_id"] != registry_id:
            raise ValueError(f"registry protocol id differs from source order: {source.resource_id}")
        if row.get("default") != source.default_id:
            raise ValueError(f"registry default differs from source: {source.resource_id}")
        entries = row["entries"]
        if not isinstance(entries, dict) or not entries:
            raise ValueError(f"missing registry entries: {source.resource_id}")
        ordered = [None] * len(entries)
        for resource_id, entry in entries.items():
            if resource_id_pattern.fullmatch(resource_id) is None:
                raise ValueError(f"invalid element resource id {resource_id!r}")
            if not isinstance(entry, dict) or set(entry) != {"protocol_id"}:
                raise ValueError(f"invalid element report row {resource_id}")
            raw_id = entry["protocol_id"]
            if type(raw_id) is not int or not 0 <= raw_id < len(ordered):
                raise ValueError(f"out-of-range element protocol id: {resource_id}")
            if ordered[raw_id] is not None:
                raise ValueError(f"duplicate element protocol id: {resource_id}")
            ordered[raw_id] = resource_id
        if source.default_id is not None and source.default_id not in entries:
            raise ValueError(f"default element absent: {source.resource_id}")
        result.append(StaticRegistryElements(source.resource_id, tuple(ordered)))
    return result


def registry_data_catalog(
    loader_source: JavaSource,
    registries_source: JavaSource,
    field_name: str,
) -> list[RegistryDataCatalogEntry]:
    registry_ids = _registry_resource_ids(registries_source)
    fields = [field for field in loader_source.fields if field.name == field_name]
    if len(fields) != 1 or fields[0].initializer is None:
        raise JavaSourceError(
            f"{loader_source.relative_path}: expected one initialized {field_name} field"
        )
    initializer = expression_fact(loader_source, fields[0].initializer)
    if (
        initializer.get("kind") != "method_invocation"
        or initializer.get("name") != "of"
        or initializer.get("object", {}).get("canonical") != "List"
    ):
        raise JavaSourceError(
            f"{loader_source.relative_path}: {field_name} must use List.of"
        )

    entries = []
    for value in initializer["arguments"]:
        arguments = value.get("arguments", [])
        if (
            value.get("kind") != "object_creation_expression"
            or value.get("name") != "RegistryDataLoader.RegistryData<>"
            or len(arguments) not in {2, 3}
        ):
            raise JavaSourceError(
                f"{loader_source.relative_path}: unsupported {field_name} entry {value.get('canonical')}"
            )
        registry = arguments[0]
        if (
            registry.get("kind") != "field_access"
            or registry.get("object", {}).get("canonical") != "Registries"
        ):
            raise JavaSourceError(
                f"{loader_source.relative_path}: registry entry must reference Registries"
            )
        constant_name = registry["name"]
        try:
            resource_id = registry_ids[constant_name]
        except KeyError as error:
            raise JavaSourceError(
                f"{registries_source.relative_path}: missing registry key {constant_name}"
            ) from error
        validator = arguments[2]["canonical"] if len(arguments) == 3 else None
        entries.append(
            RegistryDataCatalogEntry(
                constant_name=constant_name,
                resource_id=resource_id,
                codec=arguments[1]["canonical"],
                validator=validator,
            )
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise JavaSourceError(
            f"{loader_source.relative_path}: duplicate registry in {field_name}"
        )
    return entries


def built_in_registry_catalog(
    built_in_source: JavaSource,
    registries_source: JavaSource,
) -> list[BuiltInRegistryCatalogEntry]:
    registry_ids = _registry_resource_ids(registries_source)
    registration_argument_counts = {
        "registerSimple": 2,
        "registerSimpleWithIntrusiveHolders": 2,
        "registerDefaulted": 3,
        "registerDefaultedWithIntrusiveHolders": 3,
    }
    entries = []
    for field in built_in_source.fields:
        if field.initializer is None:
            continue
        initializer = expression_fact(built_in_source, field.initializer)
        registration = initializer.get("name")
        if (
            initializer.get("kind") == "method_invocation"
            and isinstance(registration, str)
            and registration.startswith("register")
            and registration not in registration_argument_counts
        ):
            raise JavaSourceError(
                f"{built_in_source.relative_path}: unsupported registry helper {registration}"
            )
        if registration not in registration_argument_counts:
            continue
        arguments = initializer.get("arguments", [])
        if (
            initializer.get("kind") != "method_invocation"
            or len(arguments) != registration_argument_counts[registration]
        ):
            raise JavaSourceError(
                f"{built_in_source.relative_path}: malformed {registration} field {field.name}"
            )
        registry = arguments[0]
        if (
            registry.get("kind") != "field_access"
            or registry.get("object", {}).get("canonical") != "Registries"
        ):
            raise JavaSourceError(
                f"{built_in_source.relative_path}: {field.name} must reference Registries"
            )
        registry_constant_name = registry["name"]
        try:
            resource_id = registry_ids[registry_constant_name]
        except KeyError as error:
            raise JavaSourceError(
                f"{registries_source.relative_path}: missing registry key {registry_constant_name}"
            ) from error
        default_id = None
        if registration.startswith("registerDefaulted"):
            if arguments[1].get("kind") != "string_literal":
                raise JavaSourceError(
                    f"{built_in_source.relative_path}: {field.name} default must be a string literal"
                )
            default_id = f"minecraft:{ast.literal_eval(arguments[1]['canonical'])}"
        entries.append(
            BuiltInRegistryCatalogEntry(
                constant_name=field.name,
                registry_constant_name=registry_constant_name,
                resource_id=resource_id,
                registration=registration,
                default_id=default_id,
            )
        )
    if len({entry.constant_name for entry in entries}) != len(entries):
        raise JavaSourceError(
            f"{built_in_source.relative_path}: duplicate built-in registry field"
        )
    if len({entry.resource_id for entry in entries}) != len(entries):
        raise JavaSourceError(
            f"{built_in_source.relative_path}: duplicate built-in registry resource id"
        )
    return entries


def _registry_resource_ids(source: JavaSource) -> dict[str, str]:
    result = {}
    for field in source.fields:
        if field.initializer is None:
            continue
        initializer = expression_fact(source, field.initializer)
        arguments = initializer.get("arguments", [])
        if (
            initializer.get("kind") != "method_invocation"
            or initializer.get("name") != "createRegistryKey"
            or len(arguments) != 1
            or arguments[0].get("kind") != "string_literal"
        ):
            continue
        path = ast.literal_eval(arguments[0]["canonical"])
        result[field.name] = f"minecraft:{path}"
    return result
