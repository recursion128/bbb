"""Deterministic frontend for checked-in vanilla data-pack tag sources."""

from __future__ import annotations

import json
from pathlib import Path


class VanillaTagSourceError(ValueError):
    """Explicit unsupported or inconsistent vanilla tag source."""


def load_tag_resources(
    source_root: Path, relative_directory: Path, *, namespace: str = "minecraft"
) -> dict[str, tuple[str, bytes]]:
    directory = source_root / relative_directory
    if not directory.is_dir():
        raise VanillaTagSourceError(
            f"{relative_directory.as_posix()}: vanilla tag directory does not exist"
        )
    return {
        f"{namespace}:" + path.relative_to(directory).with_suffix("").as_posix(): (
            path.relative_to(source_root).as_posix(),
            path.read_bytes(),
        )
        for path in sorted(directory.rglob("*.json"))
    }


def resolve_tag_values(
    tag_resources: dict[str, tuple[str, bytes]],
    tag_id: str,
    expected_count: int,
    *,
    registry_name: str,
) -> tuple[str, ...]:
    parsed: dict[str, list[str]] = {}
    for resource_id, (relative_path, raw) in sorted(tag_resources.items()):
        try:
            root = json.loads(raw)
        except (json.JSONDecodeError, UnicodeDecodeError) as error:
            raise VanillaTagSourceError(
                f"{relative_path}: invalid {registry_name} tag JSON: {error}"
            ) from error
        replace = root.get("replace", False) if isinstance(root, dict) else None
        if (
            not isinstance(root, dict)
            or set(root) - {"replace", "values"}
            or not isinstance(replace, bool)
        ):
            raise VanillaTagSourceError(
                f"{relative_path}: unsupported {registry_name} tag root"
            )
        values = root.get("values")
        if not isinstance(values, list):
            raise VanillaTagSourceError(
                f"{relative_path}: {registry_name} tag values is not an array"
            )
        parsed_values: list[str] = []
        for index, value in enumerate(values):
            if not isinstance(value, str) or not value.removeprefix("#").startswith(
                "minecraft:"
            ):
                raise VanillaTagSourceError(
                    f"{relative_path}: unsupported {registry_name} tag value at index {index}"
                )
            parsed_values.append(value)
        parsed[resource_id] = parsed_values

    resolving: list[str] = []

    def resolve(resource_id: str) -> list[str]:
        if resource_id in resolving:
            raise VanillaTagSourceError(
                f"{registry_name} tag reference cycle: "
                f"{' -> '.join([*resolving, resource_id])}"
            )
        values = parsed.get(resource_id)
        if values is None:
            raise VanillaTagSourceError(
                f"missing required {registry_name} tag reference {resource_id}"
            )
        resolving.append(resource_id)
        resolved: list[str] = []
        for value in values:
            resolved.extend(resolve(value[1:]) if value.startswith("#") else [value])
        resolving.pop()
        return resolved

    entries = tuple(dict.fromkeys(resolve(tag_id)))
    if len(entries) != expected_count:
        relative_path = tag_resources.get(tag_id, (tag_id, b""))[0]
        raise VanillaTagSourceError(
            f"{relative_path}: expected {expected_count} unique resolved {registry_name} "
            f"tag values, extracted {len(entries)}"
        )
    return entries
