"""Tree-sitter-backed extraction of the Vanilla key-binding catalog.

Two source facts decide how ``key_<name>`` lines in ``options.txt`` are read
back. ``InputConstants.Type``'s static initializer names every well-known key,
which is exactly the text ``KeyMapping.saveString`` writes. ``Options``' field
initializers declare every ``KeyMapping``, giving the binding name, its default
key and the category the key-binds screen groups it under. Both stay derived
from source instead of copied into Rust by hand.
"""

from __future__ import annotations

from dataclasses import dataclass

from vanilla_java_source import JavaSource, descendants


class KeyMappingCatalogExtractionError(ValueError):
    pass


#: `InputConstants.Type`'s three constants, in declaration order.
KEY_TYPES = ("KEYSYM", "SCANCODE", "MOUSE")

#: The `KeyMapping` constructors `Options` uses, mapped to whether the binding
#: is a `ToggleKeyMapping`.
KEY_MAPPING_CONSTRUCTORS = {
    "KeyMapping": False,
    "ToggleKeyMapping": True,
}


@dataclass(frozen=True)
class NamedKey:
    name: str
    key_type: str
    value: int


@dataclass(frozen=True)
class KeyMappingEntry:
    order: int
    field: str
    name: str
    default_type: str
    default_value: int
    category: str
    toggle: bool
    sort_order: int


def _int_literal(text: str) -> int:
    # `canonical_expression` puts a space between a unary minus and its operand.
    compact = text.replace(" ", "")
    try:
        return int(compact, 0)
    except ValueError as error:
        raise KeyMappingCatalogExtractionError(
            f"expected an integer literal, got {text}"
        ) from error


def _string_literal(text: str) -> str:
    if len(text) >= 2 and text.startswith('"') and text.endswith('"'):
        return text[1:-1]
    raise KeyMappingCatalogExtractionError(
        f"expected a Java string literal, got {text}"
    )


def named_keys(source: JavaSource) -> list[NamedKey]:
    """Every `addKey` call in `InputConstants.Type`'s static initializer."""

    keys: list[NamedKey] = []
    for call in source.calls:
        if call.name != "addKey" or call.qualifier is not None:
            continue
        if len(call.arguments) != 3:
            raise KeyMappingCatalogExtractionError(
                f"addKey takes three arguments, got {len(call.arguments)}"
            )
        key_type, name, value = (argument.canonical for argument in call.arguments)
        if key_type not in KEY_TYPES:
            raise KeyMappingCatalogExtractionError(
                f"unknown InputConstants.Type constant: {key_type}"
            )
        keys.append(NamedKey(_string_literal(name), key_type, _int_literal(value)))
    if not keys:
        raise KeyMappingCatalogExtractionError(
            "InputConstants.Type declares no named keys"
        )
    names = [key.name for key in keys]
    if len(set(names)) != len(names):
        raise KeyMappingCatalogExtractionError("InputConstants names a key twice")
    return keys


def _constructor_arguments(source: JavaSource, node) -> tuple[str, tuple[str, ...]]:
    type_node = node.child_by_field_name("type")
    arguments = node.child_by_field_name("arguments")
    if type_node is None or arguments is None:
        raise KeyMappingCatalogExtractionError("malformed object creation")
    return (
        source.text(type_node),
        tuple(
            source.expression(child).canonical
            for child in arguments.named_children
            if child.type != "comment"
        ),
    )


def _category(text: str) -> str:
    prefix = "KeyMapping.Category."
    if not text.startswith(prefix):
        raise KeyMappingCatalogExtractionError(
            f"expected a KeyMapping.Category constant, got {text}"
        )
    return text[len(prefix) :].lower()


def _default_key(arguments: tuple[str, ...]) -> tuple[str, int, tuple[str, ...]]:
    """Split the leading `(name, [type,] value)` off a KeyMapping constructor."""

    if arguments[1].startswith("InputConstants.Type."):
        key_type = arguments[1][len("InputConstants.Type.") :]
        if key_type not in KEY_TYPES:
            raise KeyMappingCatalogExtractionError(
                f"unknown InputConstants.Type constant: {arguments[1]}"
            )
        return key_type, _int_literal(arguments[2]), arguments[3:]
    if arguments[1].replace(" ", "") == "InputConstants.UNKNOWN.getValue()":
        return "KEYSYM", -1, arguments[2:]
    return "KEYSYM", _int_literal(arguments[1]), arguments[2:]


def key_mappings(source: JavaSource) -> list[KeyMappingEntry]:
    """Every `KeyMapping` `Options` constructs, in declaration order."""

    entries: list[KeyMappingEntry] = []
    for item in source.fields:
        if item.initializer is None:
            continue
        if not item.type_spelling.startswith("KeyMapping"):
            continue
        initializer = item.initializer.node
        for node in [initializer, *descendants(initializer)]:
            if node.type != "object_creation_expression":
                continue
            type_name, arguments = _constructor_arguments(source, node)
            toggle = KEY_MAPPING_CONSTRUCTORS.get(type_name)
            if toggle is None:
                raise KeyMappingCatalogExtractionError(
                    f"unknown KeyMapping constructor: {type_name}"
                )
            if len(arguments) < 3:
                raise KeyMappingCatalogExtractionError(
                    f"{type_name} takes at least three arguments, got {arguments}"
                )
            name = _string_literal(arguments[0])
            key_type, value, rest = _default_key(arguments)
            category = _category(rest[0])
            sort_order = 0
            if not toggle and len(rest) > 1:
                sort_order = _int_literal(rest[1])
            entries.append(
                KeyMappingEntry(
                    order=len(entries),
                    field=item.name,
                    name=name,
                    default_type=key_type,
                    default_value=value,
                    category=category,
                    toggle=toggle,
                    sort_order=sort_order,
                )
            )
    if not entries:
        raise KeyMappingCatalogExtractionError("Options declares no key mappings")
    names = [entry.name for entry in entries]
    if len(set(names)) != len(names):
        raise KeyMappingCatalogExtractionError("Options declares a binding twice")
    return entries
