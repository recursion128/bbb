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

#: `OptionInstance` value sets, keyed by the leading source text, mapped to the
#: JSON shape their `codec()` writes into `options.txt`.
VALUE_SET_ENCODINGS = {
    "OptionInstance.BOOLEAN_VALUES": "boolean",
    "OptionInstance.UnitDouble.INSTANCE.xmap": "double",
    "OptionInstance.UnitDouble.INSTANCE": "double",
    "new OptionInstance.IntRange": "integer",
    "new OptionInstance.ClampingLazyMaxIntRange": "integer",
    "new OptionInstance.Enum": "enum",
    "new OptionInstance.SliderableEnum": "enum",
    "new OptionInstance.LazyEnum": "enum",
}

#: The seven-argument constructor overrides the value set's codec, so the
#: explicit argument decides the encoding.
EXPLICIT_CODEC_ENCODINGS = {
    "Codec.doubleRange": "double",
    "Codec.DOUBLE": "double",
    "Codec.intRange": "integer",
    "Codec.INT": "integer",
    "GraphicsPreset.CODEC": "enum-name",
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
    encoding: str
    #: The seven-argument constructor's explicit codec, when there is one. A
    #: codec that `xmap`s stores a different value than it writes.
    codec: str
    default: str
    #: `processDumpedOptions` names these, so `dumpOptionsForReport` carries
    #: them into the crash report without naming them again.
    dumped: bool


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


def _option_instance_arguments(source: JavaSource, field: JavaField, node) -> list:
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
    return arguments


def _option_instance_default(source: JavaSource, field: JavaField, node) -> str:
    return source.text(_option_instance_arguments(source, field, node)[-1])


def _option_instance_codec(source: JavaSource, field: JavaField, node) -> str:
    if node.type == "method_invocation":
        return ""
    arguments = _option_instance_arguments(source, field, node)
    return _collapse(source.text(arguments[4])) if len(arguments) == 6 else ""


def _collapse(text: str) -> str:
    return " ".join(text.split())


def _option_instance_encoding(source: JavaSource, field: JavaField, node) -> str:
    """The on-disk encoding `save` writes for one ``OptionInstance`` line.

    ``FieldAccess.process(String, OptionInstance)`` runs the option's codec
    through ``JsonOps``, so the encoding is whatever the value set (or the
    explicit codec argument, in the seven-argument form) produces.
    """

    if node.type == "method_invocation":
        return "boolean"
    arguments = _option_instance_arguments(source, field, node)
    # (caption, tooltip, label, valueSet, [codec,] default)
    if len(arguments) == 5:
        codec_text = None
    elif len(arguments) == 6:
        codec_text = _collapse(source.text(arguments[4]))
    else:
        _fail(source, node, f"OptionInstance {field.name} has an unexpected arity")
    value_set = _collapse(source.text(arguments[3]))
    if codec_text is not None:
        for prefix, encoding in EXPLICIT_CODEC_ENCODINGS.items():
            if codec_text.startswith(prefix):
                return encoding
        _fail(source, node, f"unmapped explicit codec for {field.name}: {codec_text}")
    for prefix, encoding in VALUE_SET_ENCODINGS.items():
        if value_set.startswith(prefix):
            if encoding != "enum":
                return encoding
            return _enum_encoding(source, field, arguments[3])
    _fail(source, node, f"unmapped value set for {field.name}: {value_set}")


def _enum_encoding(source: JavaSource, field: JavaField, value_set_node) -> str:
    """`OptionInstance.Enum` carries the enum's own codec as its last argument:
    `LEGACY_CODEC` is `Codec.INT` over the id, `CODEC` is
    `StringRepresentable.fromEnum`, and `renderClouds` accepts either its name
    or the pre-1.20 boolean through `Codec.withAlternative`."""

    arguments = value_set_node.child_by_field_name("arguments")
    if arguments is None or not arguments.named_children:
        _fail(source, value_set_node, f"enum value set for {field.name} has no codec")
    codec_text = _collapse(source.text(arguments.named_children[-1]))
    if codec_text.startswith("Codec.withAlternative("):
        inner = codec_text.removeprefix("Codec.withAlternative(").split(",")
        if len(inner) >= 2 and inner[0].endswith(".CODEC") and inner[1].strip() == "Codec.BOOL":
            return "enum-name-or-boolean"
        _fail(source, value_set_node, f"unmapped alternative codec: {codec_text}")
    if codec_text == "Codec.STRING":
        # `soundDevice` is a LazyEnum over the runtime device names.
        return "string"
    if codec_text.endswith(".LEGACY_CODEC"):
        return "enum-id"
    if codec_text.endswith(".CODEC"):
        return "enum-name"
    _fail(
        source,
        value_set_node,
        f"enum value set for {field.name} has no known codec: {codec_text}",
    )


def reported_option_keys(source: JavaSource) -> list[str]:
    """The keys `dumpOptionsForReport` adds on top of `processDumpedOptions`.

    They are spelled out as `optionsForReport.add(Pair.of("name", ...))`, so the
    list stays derived instead of copied into Rust.
    """

    methods = [
        method
        for method in source.methods
        if method.name == "dumpOptionsForReport" and method.owner.endswith("Options")
    ]
    if len(methods) != 1:
        raise ClientOptionsCatalogExtractionError(
            f"expected exactly one Options#dumpOptionsForReport, found {len(methods)}"
        )
    body = methods[0].node.child_by_field_name("body")
    keys: list[str] = []
    for node in descendants(body):
        if node.type != "method_invocation":
            continue
        if source.text(node.child_by_field_name("name")) != "of":
            continue
        arguments = node.child_by_field_name("arguments")
        if arguments is None or not arguments.named_children:
            continue
        key = _string_literal(source, arguments.named_children[0])
        if key is not None:
            keys.append(key)
    if not keys:
        raise ClientOptionsCatalogExtractionError(
            "dumpOptionsForReport names no additional keys"
        )
    return keys


def _enum_declaration(source: JavaSource, simple_name: str):
    for declaration in descendants(source.root_node):
        if declaration.type != "enum_declaration":
            continue
        name_node = declaration.child_by_field_name("name")
        if name_node is not None and source.text(name_node) == simple_name:
            return declaration
    raise ClientOptionsCatalogExtractionError(
        f"{source.relative_path} declares no enum {simple_name}"
    )


def _accessor_field(source: JavaSource, declaration, accessor: str) -> str:
    """The field the on-disk accessor returns, so the value is read from the
    constructor rather than guessed from the constant's argument order."""

    for node in descendants(declaration):
        if node.type != "method_declaration":
            continue
        name_node = node.child_by_field_name("name")
        if name_node is None or source.text(name_node) != accessor:
            continue
        body = node.child_by_field_name("body")
        if body is None or len(body.named_children) != 1:
            break
        statement = body.named_children[0]
        if statement.type != "return_statement" or not statement.named_children:
            break
        returned = source.text(statement.named_children[0])
        if returned.startswith("this."):
            return returned.removeprefix("this.")
        break
    raise ClientOptionsCatalogExtractionError(
        f"enum {accessor} is not a direct field return"
    )


def _constructor_parameter_index(source: JavaSource, declaration, field_name: str) -> int:
    for node in descendants(declaration):
        if node.type != "constructor_declaration":
            continue
        parameters = node.child_by_field_name("parameters")
        if parameters is None:
            continue
        names = []
        for parameter in parameters.named_children:
            parameter_name = parameter.child_by_field_name("name")
            names.append(source.text(parameter_name) if parameter_name else "")
        if field_name in names:
            return names.index(field_name)
    raise ClientOptionsCatalogExtractionError(
        f"enum constructor takes no {field_name} parameter"
    )


def _enum_constant_argument(source: JavaSource, declaration, constant: str, index: int) -> str:
    for node in descendants(declaration):
        if node.type != "enum_constant":
            continue
        name_node = node.child_by_field_name("name")
        if name_node is None or source.text(name_node) != constant:
            continue
        arguments = node.child_by_field_name("arguments")
        if arguments is None or len(arguments.named_children) <= index:
            break
        return source.text(arguments.named_children[index])
    raise ClientOptionsCatalogExtractionError(
        f"enum constant {constant} has no argument at index {index}"
    )


def resolve_enum_default(
    load_source, options: JavaSource, encoding: str, java_default: str
) -> str:
    """Turn ``GraphicsPreset.FANCY`` into the text ``save`` writes for it.

    ``LEGACY_CODEC`` is ``Codec.INT`` over the enum's ``id`` field, ``CODEC`` is
    ``StringRepresentable.fromEnum``, and ``tutorialStep`` writes through
    ``TutorialSteps::getName`` — so every value lives in the enum's own
    constructor, never in a table copied alongside the option.
    """

    qualifier, _, constant = java_default.rpartition(".")
    simple_name = qualifier.rpartition(".")[2]
    source = load_source(qualifier, options.imports)
    declaration = _enum_declaration(source, simple_name)
    if encoding == "enum-id":
        field_name = "id"
    else:
        accessor = "getName" if encoding == "tutorial-step" else "getSerializedName"
        field_name = _accessor_field(source, declaration, accessor)
    index = _constructor_parameter_index(source, declaration, field_name)
    return _enum_constant_argument(source, declaration, constant, index)


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
        dumped = method_name == "processDumpedOptions"
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
                encoding = kind
                codec = ""
                default = _scalar_default(source, field, initializer(field))
            elif field.type_spelling.startswith("OptionInstance"):
                kind = "codec"
                encoding = _option_instance_encoding(
                    source, field, initializer(field)
                )
                codec = _option_instance_codec(source, field, initializer(field))
                default = _option_instance_default(source, field, initializer(field))
            else:
                kind = SCALAR_KINDS.get(field.type_spelling)
                if kind is None:
                    _fail(
                        source,
                        arguments[1],
                        f"process target {field_name} has unsupported type {field.type_spelling}",
                    )
                encoding = kind
                codec = ""
                default = _scalar_default(source, field, initializer(field))
            options.append(
                ClientOption(
                    order=order,
                    key=key,
                    field=field_name,
                    kind=kind,
                    encoding=encoding,
                    codec=codec,
                    default=COMPUTED_DEFAULTS.get(default, default),
                    dumped=dumped,
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
