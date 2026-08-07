#!/usr/bin/env python3
"""Generate the Vanilla 26.1 ``options.txt`` key catalog."""

from __future__ import annotations

import argparse
import difflib
import json
from dataclasses import replace
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root
import sys

from client_options_catalog_extractor import (
    ClientOptionsCatalogExtractionError,
    client_option_entries,
    reported_option_keys,
    resolve_enum_default,
)
from client_options_datafix_extractor import client_options_datafix_catalog
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO_ROOT / "docs/evidence/vanilla-26.1/client-options-catalog.json"
DEFAULT_RUST_OUTPUT = (
    REPO_ROOT / "crates/bbb-native/src/client_options/catalog_26_1.rs"
)
DEFAULT_DATAFIX_RUST_OUTPUT = (
    REPO_ROOT / "crates/bbb-native/src/client_options/datafix_26_1.rs"
)
OPTIONS_SOURCE = "net/minecraft/client/Options.java"

#: catalog `encoding` -> the Rust variant that decodes and re-encodes it.
RUST_ENCODINGS = {
    "boolean": "Boolean",
    "double": "Double",
    "integer": "Integer",
    "float": "Float",
    "string": "Text",
    "string-list": "TextList",
    "enum-id": "EnumId",
    "enum-name": "EnumName",
    "enum-name-or-boolean": "EnumNameOrBoolean",
    "tutorial-step": "TutorialStep",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--rust-output", type=Path, default=DEFAULT_RUST_OUTPUT)
    parser.add_argument(
        "--datafix-rust-output", type=Path, default=DEFAULT_DATAFIX_RUST_OUTPUT
    )
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    args.source_root = resolve_source_root(args)
    return args


def _enum_source_loader(source_root: Path, package: str):
    """Resolve the file that declares an enum named in `Options`.

    A nested enum (`MusicManager.MusicFrequency`) lives in its imported owner's
    file, an imported enum in its own, and everything else in `Options`' own
    package.
    """

    cache: dict[str, object] = {}

    def load(qualifier: str, imports: dict[str, str]):
        if qualifier in cache:
            return cache[qualifier]
        outer = qualifier.split(".")[0]
        qualified = imports.get(outer)
        if qualified is None:
            qualified = f"{package}.{outer}"
        relative = qualified.replace(".", "/") + ".java"
        path = source_root / relative
        if not path.is_file():
            raise ClientOptionsCatalogExtractionError(
                f"cannot locate the source that declares {qualifier}"
            )
        parsed = parse_source(relative, path.read_bytes())
        cache[qualifier] = parsed
        return parsed

    return load


def catalog_entries(source_root: Path):
    raw = (source_root / OPTIONS_SOURCE).read_bytes()
    source = parse_source(OPTIONS_SOURCE, raw)
    options, groups = client_option_entries(source)
    reported = reported_option_keys(source)
    load_enum = _enum_source_loader(source_root, source.package)
    options = [replace(option, default=_written_default(load_enum, source, option))
               for option in options]
    return options, groups, reported


#: `fov` is the one option whose explicit codec stores a different number than
#: it writes, so its default is put through the codec's write half here rather
#: than guessed. Keyed by the exact codec text and stored default.
XMAP_WRITTEN_DEFAULTS = {
    (
        "Codec.DOUBLE.xmap(value -> (int)(value * 40.0 + 70.0),"
        " value -> (value.intValue() - 70.0) / 40.0)",
        "70",
    ): "0.0",
}


def _java_string_text(literal: str) -> str:
    if literal.startswith('"') and literal.endswith('"'):
        return literal[1:-1]
    raise ClientOptionsCatalogExtractionError(
        f"expected a Java string literal, got {literal}"
    )


def _written_default(load_enum, source, option) -> str:
    """The exact text `Options.save` puts after the `key:`.

    The `OptionInstance` overload runs the value through GSON, so strings keep
    their quotes; every other overload is `writer.println(value)`, which writes
    the bare value.
    """

    default = option.default
    if ".xmap(" in option.codec:
        written = XMAP_WRITTEN_DEFAULTS.get((option.codec, default))
        if written is None:
            raise ClientOptionsCatalogExtractionError(
                f"{option.key} stores a value its codec rewrites; declare the "
                f"written default for {option.codec} at {default}"
            )
        return written
    if option.encoding in {"enum-id", "enum-name", "enum-name-or-boolean"}:
        resolved = resolve_enum_default(load_enum, source, option.encoding, default)
        # GSON quotes the serialized name; the id is written as a bare int.
        return resolved
    if option.encoding == "tutorial-step":
        return _java_string_text(
            resolve_enum_default(load_enum, source, option.encoding, default)
        )
    if option.kind == "codec":
        return default
    if option.kind == "string":
        return _java_string_text(default)
    if option.kind == "float" and default.endswith(("F", "f")):
        return default[:-1]
    return default


HOST_DEFAULT_VARIANTS = {
    "host:locale-is-japanese": "LocaleIsJapanese",
    "host:platform-is-windows": "PlatformIsWindows",
}


def _rust_default(option) -> str:
    variant = HOST_DEFAULT_VARIANTS.get(option.default)
    if variant is not None:
        return f"ClientOptionDefault::{variant}"
    escaped = option.default.replace("\\", "\\\\").replace('"', '\\"')
    return f'ClientOptionDefault::Written("{escaped}")'


def render_rust_catalog(options, groups, reported) -> str:
    lines = [
        "// @generated by scripts/generate-client-options-catalog.py; do not edit manually.",
        "// Source: Vanilla 26.1 Options.processDumpedOptions then Options.processOptions,",
        "// which is the order Options.save writes options.txt.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-client-options-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-client-options-catalog.py --check",
        "",
        "/// The shape `Options.save` writes for one line, and therefore the shape",
        "/// `Options.load` accepts back.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub(crate) enum ClientOptionEncoding {",
    ]
    used = list(dict.fromkeys(RUST_ENCODINGS[option.encoding] for option in options))
    lines.extend(f"    {variant}," for variant in used)
    lines.extend(
        [
            "}",
            "",
            "/// A default Vanilla writes verbatim, or one the running host decides.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(crate) enum ClientOptionDefault {",
            "    /// Exactly the text `save` writes when the value is untouched.",
            "    Written(&'static str),",
            "    /// `Locale.getDefault().getLanguage().equalsIgnoreCase(\"ja\")`.",
            "    LocaleIsJapanese,",
            "    /// `Util.getPlatform() == Util.OS.WINDOWS`.",
            "    PlatformIsWindows,",
            "}",
            "",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(crate) struct ClientOptionSpec {",
            "    pub(crate) key: &'static str,",
            "    pub(crate) encoding: ClientOptionEncoding,",
            "    pub(crate) default: ClientOptionDefault,",
            "    /// `processDumpedOptions` names this key, so the crash report",
            "    /// carries it.",
            "    pub(crate) dumped: bool,",
            "}",
            "",
            "/// Every fixed `options.txt` line, in `Options.save` order.",
            f"pub(crate) const CLIENT_OPTIONS: [ClientOptionSpec; {len(options)}] = [",
        ]
    )
    for option in options:
        lines.extend(
            [
                "    ClientOptionSpec {",
                f'        key: "{option.key}",',
                f"        encoding: ClientOptionEncoding::{RUST_ENCODINGS[option.encoding]},",
                f"        default: {_rust_default(option)},",
                f"        dumped: {str(option.dumped).lower()},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "/// The three trailing loops write one line per runtime element, so their",
            "/// keys are a prefix plus the element's own name.",
            (
                f"pub(crate) const CLIENT_OPTION_PREFIXES: [&str; {len(groups)}] = ["
                + ", ".join(f'\"{group.prefix}\"' for group in groups)
                + "];"
            ),
            "",
        ]
    )
    lines.extend(
        [
            "/// Keys `dumpOptionsForReport` names on top of the dumped block.",
            f"pub(crate) const CLIENT_OPTION_REPORT_EXTRA_KEYS: [&str; {len(reported)}] = [",
        ]
    )
    lines.extend(f'    "{key}",' for key in reported)
    lines.append("];")
    lines.append("")
    return "\n".join(lines)


RUST_DATA_FIX_KINDS = {
    "force-vbo": "ForceVbo",
    "lower-case-language": "LowerCaseLanguage",
    "key-lwjgl3": "KeyLwjgl3",
    "key-translation": "KeyTranslation",
    "add-text-background": "AddTextBackground",
    "fancy-graphics-to-mode": "FancyGraphicsToMode",
    "rename-field": "RenameField",
    "programmer-art": "ProgrammerArt",
    "ambient-occlusion": "AmbientOcclusion",
    "accessibility-onboard": "AccessibilityOnboard",
    "menu-blurriness": "MenuBlurriness",
    "graphics-mode-split": "GraphicsModeSplit",
    "graphics-preset-custom": "GraphicsPresetCustom",
    "music-toast": "MusicToast",
}


def _rust_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def _rust_data_fix_kind(fix) -> str:
    variant = RUST_DATA_FIX_KINDS[fix.kind]
    if fix.kind == "rename-field":
        source, target = fix.arguments
        return (
            f"ClientOptionsDataFixKind::{variant} {{\n"
            f"            source: {_rust_string(source)},\n"
            f"            target: {_rust_string(target)},\n"
            "        }"
        )
    if fix.kind == "graphics-mode-split":
        field, fast, fancy, fabulous = fix.arguments
        return (
            f"ClientOptionsDataFixKind::{variant} {{\n"
            f"            field: {_rust_string(field)},\n"
            f"            fast: {_rust_string(fast)},\n"
            f"            fancy: {_rust_string(fancy)},\n"
            f"            fabulous: {_rust_string(fabulous)},\n"
            "        }"
        )
    return f"ClientOptionsDataFixKind::{variant}"


def render_rust_datafix(catalog) -> str:
    lines = [
        "// @generated by scripts/generate-client-options-catalog.py; do not edit manually.",
        "// Sources: Vanilla 26.1 DataFixers registrations and Options*Fix classes.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-client-options-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-client-options-catalog.py --check",
        "",
        "use super::datafix::{ClientOptionsDataFix, ClientOptionsDataFixKind};",
        "",
        "/// `DataFixTypes.OPTIONS` registrations in DataFixer execution order.",
        f"pub(super) const CLIENT_OPTIONS_DATA_FIXES: [ClientOptionsDataFix; {len(catalog.fixes)}] = [",
    ]
    for fix in catalog.fixes:
        lines.extend(
            [
                "    ClientOptionsDataFix {",
                f"        version: {fix.version},",
                f"        kind: {_rust_data_fix_kind(fix)},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            "/// `OptionsKeyLwjgl3Fix.MAP`: legacy LWJGL 2 code to old key name.",
            f"pub(super) const LEGACY_LWJGL_KEY_NAMES: [(i32, &str); {len(catalog.legacy_lwjgl_keys)}] = [",
        ]
    )
    lines.extend(
        f"    ({code}, {_rust_string(name)}),"
        for code, name in catalog.legacy_lwjgl_keys
    )
    lines.extend([
        "];",
        "",
    ])
    return "\n".join(lines)


def render_catalog(source_root: Path) -> str:
    options, groups, reported = catalog_entries(source_root)
    datafix = client_options_datafix_catalog(source_root)
    document = {
        "generated_by": "scripts/generate-client-options-catalog.py",
        "vanilla_version": "26.1",
        "source_file": OPTIONS_SOURCE,
        "line_format": "name:value, split on the first ':' only; bad lines are skipped",
        "options": [
            {
                "order": option.order,
                "key": option.key,
                "field": option.field,
                "kind": option.kind,
                "encoding": option.encoding,
                "default": option.default,
                "dumped": option.dumped,
            }
            for option in options
        ],
        "dynamic_groups": [
            {"order": group.order, "prefix": group.prefix, "kind": group.kind}
            for group in groups
        ],
        "report_extra_keys": reported,
        "data_fix": {
            "owner": "net.minecraft.client.Options#dataFix(final CompoundTag tag)",
            "registration_source": "net/minecraft/util/datafix/DataFixers.java#addFixers",
            "type": "DataFixTypes.OPTIONS",
            "invalid_or_missing_version": 0,
            "application_rule": "apply each registration in order when input version is lower than the fix schema version",
            "source_files": [
                {"path": path, "sha256": sha256}
                for path, sha256 in datafix.source_hashes
            ],
            "fixes": [
                {
                    "order": fix.order,
                    "version": fix.version,
                    "kind": fix.kind,
                    "arguments": list(fix.arguments),
                    "source_file": fix.source_file,
                    "source_sha256": fix.source_sha256,
                }
                for fix in datafix.fixes
            ],
            "legacy_lwjgl_keys": [
                {"code": code, "name": name}
                for code, name in datafix.legacy_lwjgl_keys
            ],
        },
    }
    return json.dumps(document, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    args = parse_args()
    options, groups, reported = catalog_entries(args.source_root)
    datafix = client_options_datafix_catalog(args.source_root)
    outputs = [
        (args.output, render_catalog(args.source_root)),
        (args.rust_output, render_rust_catalog(options, groups, reported)),
        (args.datafix_rust_output, render_rust_datafix(datafix)),
    ]
    if args.check:
        stale = False
        for path, rendered in outputs:
            current = path.read_text(encoding="utf-8") if path.exists() else ""
            if current == rendered:
                continue
            stale = True
            sys.stdout.writelines(
                difflib.unified_diff(
                    current.splitlines(keepends=True),
                    rendered.splitlines(keepends=True),
                    fromfile=str(path),
                    tofile="generated",
                )
            )
        if stale:
            return 1
        print("client options catalog: checked output is current")
        return 0
    for path, rendered in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(rendered, encoding="utf-8")
        print(f"client options catalog: wrote {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
