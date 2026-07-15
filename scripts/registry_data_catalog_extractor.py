"""Extract RegistryDataLoader registry lists from vanilla Java CST facts."""

from __future__ import annotations

import ast
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
