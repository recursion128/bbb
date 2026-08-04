"""Tree-sitter-backed extraction of the Vanilla ``Options`` persisted fields.

``Options.processDumpedOptions`` followed by ``Options.processOptions`` is the
one place that names every ``options.txt`` key, in the order ``save`` writes
them. Each ``access.process`` call names a target, and the target's declared
type picks the ``FieldAccess`` overload — which is what decides how the value is
encoded on the line. Defaults come from the field initializers, so the catalog
stays derived from source instead of copied into Rust by hand.
"""

from __future__ import annotations

from dataclasses import dataclass

from vanilla_java_source import JavaField, JavaSource, descendants


class ClientOptionsCatalogExtractionError(ValueError):
    pass


#: ``processOptions`` reads these through the trailing reader/writer overload,
#: so the value is neither a JSON codec nor a bare scalar.
CUSTOM_CODEC_KINDS = {
    "resourcePacks": "string-list",
    "incompatibleResourcePacks": "string-list",
    "tutorialStep": "tutorial-step",
}

#: The three loops at the end of ``processOptions`` write one line per runtime
#: element rather than a fixed key.
DYNAMIC_PREFIXES = {
    '"key_" + keyMapping.getName()': "key-mapping",
    '"soundCategory_" + source.getName()': "sound-source",
    '"modelPart_" + part.getId()': "model-part",
}

#: Declared field type -> the ``FieldAccess`` overload the call resolves to.
SCALAR_KINDS = {
    "boolean": "boolean",
    "int": "integer",
    "float": "float",
    "String": "string",
}

#: Java's implicit field defaults, for the persisted primitives Vanilla leaves
#: uninitialized (`hideServerAddress`, `overrideWidth`, ...).
IMPLICIT_SCALAR_DEFAULTS = {
    "boolean": "false",
    "int": "0",
    "float": "0.0F",
}

#: Defaults Vanilla computes instead of writing as a literal. Keyed by the exact
#: source text so any upstream change to the expression fails the generator
#: rather than silently keeping a stale value. `host:` markers name a value the
#: running host decides, which the Rust catalog resolves at load time.
COMPUTED_DEFAULTS = {
    # Locale.getDefault().getLanguage().equalsIgnoreCase("ja")
    "japaneseGlyphVariantsDefault()": "host:locale-is-japanese",
    # Util.getPlatform() == Util.OS.WINDOWS
    "Util.getPlatform() == Util.OS.WINDOWS": "host:platform-is-windows",
    # 70.0 / (ChatComponent.getHeight(1.0) - 20) = 70.0 / (180 - 20)
    "ChatComponent.defaultUnfocusedPct()": "0.4375",
    # Math.pow(10.0, 0 / 100.0)
    "logMouse(0)": "1.0",
    "Lists.newArrayList()": "[]",
}


@dataclass(frozen=True)
class ClientOption:
    """One fixed ``options.txt`` line, in ``save`` order."""

    order: int
    key: str
    field: str
    kind: str
    default: str


@dataclass(frozen=True)
class DynamicClientOptionGroup:
    """One of the three per-element line groups ``processOptions`` writes."""

    order: int
    prefix: str
    kind: str


def _fail(source: JavaSource, node, message: str) -> None:
    point = node.start_point
    raise ClientOptionsCatalogExtractionError(
        f"{source.relative_path}:{point[0] + 1}:{point[1] + 1}: {message}"
    )


def _string_literal(source: JavaSource, node) -> str | None:
    if node.type != "string_literal":
        return None
    text = source.text(node)
    return text[1:-1]


def _process_calls(source: JavaSource, method_name: str) -> list:
    methods = [
        method
        for method in source.methods
        if method.name == method_name and method.owner.endswith("Options")
    ]
    if len(methods) != 1:
        raise ClientOptionsCatalogExtractionError(
            f"expected exactly one Options#{method_name}, found {len(methods)}"
        )
    body = methods[0].node.child_by_field_name("body")
    if body is None:
        raise ClientOptionsCatalogExtractionError(
            f"Options#{method_name} has no body"
        )
    return [
        node
        for node in descendants(body)
        if node.type == "method_invocation"
        and source.text(node.child_by_field_name("name")) == "process"
    ]


def _target_field_name(source: JavaSource, node) -> str:
    text = source.text(node)
    return text.split(".")[-1] if text.startswith("this.") else text


def _constructor_assignments(source: JavaSource) -> dict[str, object]:
    """``this.<field> = ...`` inside the constructor, for fields whose value
    depends on a constructor argument (``renderDistance``'s large-distance
    range)."""

    assignments: dict[str, object] = {}
    for method in source.methods:
        if not (method.is_constructor and method.owner.endswith("Options")):
            continue
        body = method.node.child_by_field_name("body")
        if body is None:
            continue
        for node in descendants(body):
            if node.type != "assignment_expression":
                continue
            left = node.child_by_field_name("left")
            right = node.child_by_field_name("right")
            if left is None or right is None:
                continue
            text = source.text(left)
            if text.startswith("this."):
                assignments.setdefault(text.removeprefix("this."), right)
    return assignments


def _option_instance_default(source: JavaSource, field: JavaField, node) -> str:
    if node.type == "method_invocation":
        name = source.text(node.child_by_field_name("name"))
        if name != "createBoolean":
            _fail(source, node, f"unsupported OptionInstance factory {name}")
    elif node.type != "object_creation_expression":
        _fail(source, node, f"unsupported OptionInstance initializer {node.type}")
    arguments = list(node.child_by_field_name("arguments").named_children)
    if not arguments:
        _fail(source, node, f"OptionInstance {field.name} has no arguments")
    # `createBoolean` and the constructor both end with an optional
    # onValueUpdate callback, so the default is the last argument that is not
    # one. A computed default (`japaneseGlyphVariantsDefault()`) stays.
    if arguments[-1].type in {"lambda_expression", "method_reference"}:
        arguments = arguments[:-1]
    if len(arguments) < 2:
        _fail(source, node, f"OptionInstance {field.name} has no default argument")
    return source.text(arguments[-1])


def _scalar_default(source: JavaSource, field: JavaField, node) -> str:
    if node is not None:
        return source.text(node)
    implicit = IMPLICIT_SCALAR_DEFAULTS.get(field.type_spelling)
    if implicit is None:
        _fail(
            source,
            field.node,
            f"persisted field {field.name} has neither an initializer nor an implicit default",
        )
    return implicit


def client_option_entries(
    source: JavaSource,
) -> tuple[list[ClientOption], list[DynamicClientOptionGroup]]:
    """Return the fixed keys and dynamic groups in ``Options.save`` order."""

    fields = {
        field.name: field
        for field in source.fields
        if field.owner.endswith("Options")
    }
    assignments = _constructor_assignments(source)

    def initializer(field: JavaField):
        if field.initializer is not None:
            return field.initializer.node
        return assignments.get(field.name)

    options: list[ClientOption] = []
    groups: list[DynamicClientOptionGroup] = []
    seen: set[str] = set()
    order = 0
    for method_name in ("processDumpedOptions", "processOptions"):
        for call in _process_calls(source, method_name):
            arguments = list(call.child_by_field_name("arguments").named_children)
            if len(arguments) not in {2, 4}:
                _fail(source, call, "process call must take 2 or 4 arguments")
            key_text = source.text(arguments[0])
            if key_text in DYNAMIC_PREFIXES:
                groups.append(
                    DynamicClientOptionGroup(
                        order=order,
                        prefix=key_text.split('"')[1],
                        kind=DYNAMIC_PREFIXES[key_text],
                    )
                )
                order += 1
                continue
            key = _string_literal(source, arguments[0])
            if key is None:
                _fail(source, arguments[0], "process key is neither a literal nor a known group")
            if key in seen:
                _fail(source, arguments[0], f"duplicate options.txt key {key}")
            seen.add(key)
            field_name = _target_field_name(source, arguments[1])
            field = fields.get(field_name)
            if field is None:
                _fail(source, arguments[1], f"process target {field_name} is not an Options field")
            if len(arguments) == 4:
                kind = CUSTOM_CODEC_KINDS.get(field_name)
                if kind is None:
                    _fail(
                        source,
                        arguments[1],
                        f"reader/writer target {field_name} has no declared codec kind",
                    )
                default = _scalar_default(source, field, initializer(field))
            elif field.type_spelling.startswith("OptionInstance"):
                kind = "codec"
                default = _option_instance_default(source, field, initializer(field))
            else:
                kind = SCALAR_KINDS.get(field.type_spelling)
                if kind is None:
                    _fail(
                        source,
                        arguments[1],
                        f"process target {field_name} has unsupported type {field.type_spelling}",
                    )
                default = _scalar_default(source, field, initializer(field))
            options.append(
                ClientOption(
                    order=order,
                    key=key,
                    field=field_name,
                    kind=kind,
                    default=COMPUTED_DEFAULTS.get(default, default),
                )
            )
            order += 1
    prefixes = [group.prefix for group in groups]
    if len(set(prefixes)) != len(prefixes):
        raise ClientOptionsCatalogExtractionError(
            f"duplicate dynamic option groups: {prefixes}"
        )
    # A newly added loop, or a key expression that is neither a literal nor a
    # declared prefix, already fails above — the caller pins the exact set.
    return options, groups
