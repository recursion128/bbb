"""Extract the Vanilla 26.1 ``DataFixTypes.OPTIONS`` migration program.

The runtime implementation deliberately stays native Rust.  This extractor
owns the source-derived registration order, schema version, constructor facts,
legacy LWJGL key table, and exact source hashes that make that implementation
stale when Mojang changes an options fix.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
from pathlib import Path

from vanilla_java_source import JavaSource, descendants, parse_source


DATA_FIXERS_SOURCE = "net/minecraft/util/datafix/DataFixers.java"
DATA_FIX_TYPES_SOURCE = "net/minecraft/util/datafix/DataFixTypes.java"
OPTIONS_SOURCE = "net/minecraft/client/Options.java"
FIX_SOURCE_ROOT = "net/minecraft/util/datafix/fixes"


class ClientOptionsDataFixExtractionError(ValueError):
    """A Vanilla options migration fact is malformed or unsupported."""


@dataclass(frozen=True)
class ClientOptionsDataFix:
    order: int
    version: int
    kind: str
    arguments: tuple[str, ...]
    source_file: str
    source_sha256: str


@dataclass(frozen=True)
class ClientOptionsDataFixCatalog:
    fixes: tuple[ClientOptionsDataFix, ...]
    legacy_lwjgl_keys: tuple[tuple[int, str], ...]
    source_hashes: tuple[tuple[str, str], ...]


# These are hashes of the verified repo-managed Vanilla 26.1 source tree.  A
# changed implementation must fail extraction until its native decision logic
# and focused tests are reviewed; merely regenerating a new hash is not enough.
EXPECTED_SOURCE_SHA256 = {
    OPTIONS_SOURCE: "c57218276e055eef1dfe8190dea329bede17b7110f453864f88f9f43ae4f140f",
    DATA_FIX_TYPES_SOURCE: "250dd482723e4c2ab42c874f1d829423039c0d1b9e87eed9785af66be42bcecb",
    DATA_FIXERS_SOURCE: "0a5eba7e5fa2aef042d2cf4b0aae78f7cc1cae6c2e85332b85ff16cbb12c523c",
    f"{FIX_SOURCE_ROOT}/OptionsAccessibilityOnboardFix.java": "78201093ca05d43b15beae3f43174fb713e02be6b2d28ed34de23c467c85c2cc",
    f"{FIX_SOURCE_ROOT}/OptionsAddTextBackgroundFix.java": "476594f2659e171f1ee8c9d742dc8f1abdbd78b97a959b832c87107b6ecad60c",
    f"{FIX_SOURCE_ROOT}/OptionsAmbientOcclusionFix.java": "9037e4e52ff923fce99d8af054ed1ec71c66f882baa543bbbd6bbca1ff8279ac",
    f"{FIX_SOURCE_ROOT}/OptionsFancyGraphicsToGraphicsModeFix.java": "cd5ee3e15bd5b15332d01ca7636c07951df1cfb5005e2471056ea3c0e187a2f8",
    f"{FIX_SOURCE_ROOT}/OptionsForceVBOFix.java": "02c6692681f0e7e11ff5785799d226c38753f0e55161b33ea327da2a79a3469c",
    f"{FIX_SOURCE_ROOT}/OptionsGraphicsModeSplitFix.java": "498c06f341397d99160dc90015c9ff9d47465ae268f6bfa288f90cb256ebcc39",
    f"{FIX_SOURCE_ROOT}/OptionsKeyLwjgl3Fix.java": "5c11c73876c3cc78a9917b1120cf2f188fb6dd6a125b6da7ce153be55ea1ee82",
    f"{FIX_SOURCE_ROOT}/OptionsKeyTranslationFix.java": "8b7ceb3a6df541883a8a67c26aa3e17508a88901591b61a031d2662caef8a99c",
    f"{FIX_SOURCE_ROOT}/OptionsLowerCaseLanguageFix.java": "d3f923bbf9efc6dd67e993ce331eab1f3e3ce0a37b4c2c27ec62d9129b68f906",
    f"{FIX_SOURCE_ROOT}/OptionsMenuBlurrinessFix.java": "29243321ef67e439c2d923d2897deea4670c53ec3fc583b58d1ff11e6a85a636",
    f"{FIX_SOURCE_ROOT}/OptionsMusicToastFix.java": "2c835a11aab792da92dbec4ff3cfc4dd3e6d73f79cd1401cf555b35f7a5affab",
    f"{FIX_SOURCE_ROOT}/OptionsProgrammerArtFix.java": "1c5fd58a022a30b1f47968318eb9cbda98211b8c410099693a37e7f72e48a05e",
    f"{FIX_SOURCE_ROOT}/OptionsRenameFieldFix.java": "a6c7ac0072dc05393d022701d00c9a1c2b2f50d4061263bd5d00aceb8e3b20ef",
    f"{FIX_SOURCE_ROOT}/OptionsSetGraphicsPresetToCustomFix.java": "8b198596460244a78b4265f4acf1696f2c96a72d3ab06e53e1ab92559418df70",
}


FIX_KINDS = {
    "OptionsForceVBOFix": "force-vbo",
    "OptionsLowerCaseLanguageFix": "lower-case-language",
    "OptionsKeyLwjgl3Fix": "key-lwjgl3",
    "OptionsKeyTranslationFix": "key-translation",
    "OptionsAddTextBackgroundFix": "add-text-background",
    "OptionsFancyGraphicsToGraphicsModeFix": "fancy-graphics-to-mode",
    "OptionsRenameFieldFix": "rename-field",
    "OptionsProgrammerArtFix": "programmer-art",
    "OptionsAmbientOcclusionFix": "ambient-occlusion",
    "OptionsAccessibilityOnboardFix": "accessibility-onboard",
    "OptionsMenuBlurrinessFix": "menu-blurriness",
    "OptionsGraphicsModeSplitFix": "graphics-mode-split",
    "OptionsSetGraphicsPresetToCustomFix": "graphics-preset-custom",
    "OptionsMusicToastFix": "music-toast",
}


def _sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def _verified_sources(source_root: Path) -> dict[str, bytes]:
    sources: dict[str, bytes] = {}
    for relative_path, expected in EXPECTED_SOURCE_SHA256.items():
        path = source_root / relative_path
        if not path.is_file():
            raise ClientOptionsDataFixExtractionError(
                f"missing Vanilla options data-fix source {relative_path}"
            )
        raw = path.read_bytes()
        actual = _sha256(raw)
        if actual != expected:
            raise ClientOptionsDataFixExtractionError(
                f"{relative_path} sha256 changed: expected {expected}, got {actual}; "
                "review the native migration decisions before updating the hash"
            )
        sources[relative_path] = raw
    return sources


def _method_body(source: JavaSource, name: str):
    methods = [method for method in source.methods if method.name == name]
    if len(methods) != 1 or methods[0].body is None:
        raise ClientOptionsDataFixExtractionError(
            f"{source.relative_path}: expected exactly one body for {name}"
        )
    body = methods[0].node.child_by_field_name("body")
    if body is None:
        raise ClientOptionsDataFixExtractionError(
            f"{source.relative_path}: {name} has no CST body"
        )
    return body


def _within(node, ancestor) -> bool:
    current = node
    while current is not None:
        if current == ancestor:
            return True
        current = current.parent
    return False


def _schema_versions(source: JavaSource, body) -> dict[str, int]:
    versions: dict[str, int] = {}
    for call in source.calls:
        if (
            call.name != "addSchema"
            or call.qualifier is None
            or call.qualifier.canonical != "fixerUpper"
            or not _within(call.node, body)
        ):
            continue
        declarator = call.node.parent
        while declarator is not None and declarator.type != "variable_declarator":
            declarator = declarator.parent
        if declarator is None:
            continue
        name_node = declarator.child_by_field_name("name")
        if name_node is None or not call.arguments:
            raise ClientOptionsDataFixExtractionError(
                f"{source.relative_path}: malformed addSchema registration"
            )
        name = source.text(name_node)
        try:
            version = int(call.arguments[0].canonical)
        except ValueError as error:
            raise ClientOptionsDataFixExtractionError(
                f"{source.relative_path}: unsupported schema version "
                f"{call.arguments[0].canonical}"
            ) from error
        versions[name] = version
    return versions


def _java_string(expression: str) -> str:
    try:
        value = json.loads(expression)
    except json.JSONDecodeError as error:
        raise ClientOptionsDataFixExtractionError(
            f"expected Java string literal, got {expression}"
        ) from error
    if not isinstance(value, str):
        raise ClientOptionsDataFixExtractionError(
            f"expected Java string literal, got {expression}"
        )
    return value


def _operation_arguments(class_name: str, raw: list[str]) -> tuple[str, ...]:
    # The first constructor argument is always the output schema.  Boolean
    # `changesType` and human-readable fix names do not affect the remainder
    # map transformation owned by options.txt.
    if class_name == "OptionsRenameFieldFix":
        if len(raw) != 5:
            raise ClientOptionsDataFixExtractionError(
                f"unexpected {class_name} arguments: {raw}"
            )
        return (_java_string(raw[3]), _java_string(raw[4]))
    if class_name == "OptionsGraphicsModeSplitFix":
        if len(raw) != 5:
            raise ClientOptionsDataFixExtractionError(
                f"unexpected {class_name} arguments: {raw}"
            )
        return tuple(_java_string(value) for value in raw[1:])
    expected_lengths = {
        "OptionsForceVBOFix": 2,
        "OptionsLowerCaseLanguageFix": 2,
        "OptionsKeyLwjgl3Fix": 2,
        "OptionsKeyTranslationFix": 2,
        "OptionsAddTextBackgroundFix": 2,
        "OptionsFancyGraphicsToGraphicsModeFix": 1,
        "OptionsProgrammerArtFix": 1,
        "OptionsAmbientOcclusionFix": 1,
        "OptionsAccessibilityOnboardFix": 1,
        "OptionsMenuBlurrinessFix": 1,
        "OptionsSetGraphicsPresetToCustomFix": 1,
        "OptionsMusicToastFix": 2,
    }
    expected = expected_lengths.get(class_name)
    if expected is None or len(raw) != expected:
        raise ClientOptionsDataFixExtractionError(
            f"unexpected {class_name} arguments: {raw}"
        )
    return ()


def _fix_registrations(
    source: JavaSource, body, schema_versions: dict[str, int], source_hashes: dict[str, str]
) -> tuple[ClientOptionsDataFix, ...]:
    result: list[ClientOptionsDataFix] = []
    for call in source.calls:
        if (
            call.name != "addFixer"
            or call.qualifier is None
            or call.qualifier.canonical != "fixerUpper"
            or len(call.arguments) != 1
            or not _within(call.node, body)
        ):
            continue
        creation = call.arguments[0].node
        if creation.type != "object_creation_expression":
            continue
        type_node = creation.child_by_field_name("type")
        arguments_node = creation.child_by_field_name("arguments")
        if type_node is None or arguments_node is None:
            raise ClientOptionsDataFixExtractionError(
                f"{source.relative_path}: malformed addFixer creation"
            )
        class_name = source.expression(type_node).canonical.replace(" ", "")
        if not class_name.startswith("Options"):
            continue
        kind = FIX_KINDS.get(class_name)
        if kind is None:
            raise ClientOptionsDataFixExtractionError(
                f"unsupported Vanilla options fix {class_name}"
            )
        raw_arguments = [
            source.expression(child).canonical for child in arguments_node.named_children
        ]
        if not raw_arguments or raw_arguments[0] not in schema_versions:
            raise ClientOptionsDataFixExtractionError(
                f"{class_name} references unknown schema {raw_arguments[:1]}"
            )
        fix_source = f"{FIX_SOURCE_ROOT}/{class_name}.java"
        result.append(
            ClientOptionsDataFix(
                order=len(result),
                version=schema_versions[raw_arguments[0]],
                kind=kind,
                arguments=_operation_arguments(class_name, raw_arguments),
                source_file=fix_source,
                source_sha256=source_hashes[fix_source],
            )
        )
    if len(result) != 17:
        raise ClientOptionsDataFixExtractionError(
            f"expected 17 Vanilla options fixes, found {len(result)}"
        )
    if any(left.version > right.version for left, right in zip(result, result[1:])):
        raise ClientOptionsDataFixExtractionError(
            "Vanilla options fixes are not in nondecreasing schema order"
        )
    return tuple(result)


def _legacy_lwjgl_keys(source: JavaSource) -> tuple[tuple[int, str], ...]:
    result: list[tuple[int, str]] = []
    for call in source.calls:
        if (
            call.name != "put"
            or call.qualifier is None
            or call.qualifier.canonical != "map"
            or len(call.arguments) != 2
        ):
            continue
        try:
            code = int(call.arguments[0].canonical)
        except ValueError as error:
            raise ClientOptionsDataFixExtractionError(
                f"unsupported LWJGL key code {call.arguments[0].canonical}"
            ) from error
        result.append((code, _java_string(call.arguments[1].canonical)))
    if len(result) != 112 or len({code for code, _ in result}) != len(result):
        raise ClientOptionsDataFixExtractionError(
            f"expected 112 unique legacy LWJGL keys, found {len(result)}"
        )
    return tuple(result)


def client_options_datafix_catalog(source_root: Path) -> ClientOptionsDataFixCatalog:
    sources = _verified_sources(source_root)
    source_hashes = tuple((path, _sha256(raw)) for path, raw in sorted(sources.items()))
    hash_by_path = dict(source_hashes)
    registrations = parse_source(DATA_FIXERS_SOURCE, sources[DATA_FIXERS_SOURCE])
    body = _method_body(registrations, "addFixers")
    schema_versions = _schema_versions(registrations, body)
    fixes = _fix_registrations(registrations, body, schema_versions, hash_by_path)
    key_source_path = f"{FIX_SOURCE_ROOT}/OptionsKeyLwjgl3Fix.java"
    key_source = parse_source(key_source_path, sources[key_source_path])
    return ClientOptionsDataFixCatalog(
        fixes=fixes,
        legacy_lwjgl_keys=_legacy_lwjgl_keys(key_source),
        source_hashes=source_hashes,
    )
