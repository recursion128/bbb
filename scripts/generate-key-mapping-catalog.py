#!/usr/bin/env python3
"""Generate the Vanilla 26.1 key-binding catalog."""

from __future__ import annotations

import argparse
import difflib
import json
from pathlib import Path
import sys

from key_mapping_catalog_extractor import key_mappings, named_keys
from vanilla_java_source import parse_source


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
DEFAULT_OUTPUT = REPO_ROOT / "docs/evidence/vanilla-26.1/key-mapping-catalog.json"
DEFAULT_RUST_OUTPUT = (
    REPO_ROOT / "crates/bbb-native/src/client_options/key_mappings_26_1.rs"
)
INPUT_CONSTANTS_SOURCE = "com/mojang/blaze3d/platform/InputConstants.java"
OPTIONS_SOURCE = "net/minecraft/client/Options.java"

RUST_KEY_TYPES = {
    "KEYSYM": "Keysym",
    "SCANCODE": "Scancode",
    "MOUSE": "Mouse",
}

#: `winit::keyboard::KeyCode` for the `key.keyboard.*` names whose spelling does
#: not follow one of the mechanical families below. A name that matches neither
#: fails the build rather than silently losing a binding.
WINIT_KEY_CODES = {
    "apostrophe": "Quote",
    "backslash": "Backslash",
    "backspace": "Backspace",
    "caps.lock": "CapsLock",
    "comma": "Comma",
    "delete": "Delete",
    "down": "ArrowDown",
    "end": "End",
    "enter": "Enter",
    "equal": "Equal",
    "escape": "Escape",
    "grave.accent": "Backquote",
    "home": "Home",
    "insert": "Insert",
    "keypad.add": "NumpadAdd",
    "keypad.decimal": "NumpadDecimal",
    "keypad.divide": "NumpadDivide",
    "keypad.enter": "NumpadEnter",
    "keypad.equal": "NumpadEqual",
    "keypad.multiply": "NumpadMultiply",
    "keypad.subtract": "NumpadSubtract",
    "left": "ArrowLeft",
    "left.alt": "AltLeft",
    "left.bracket": "BracketLeft",
    "left.control": "ControlLeft",
    "left.shift": "ShiftLeft",
    "left.win": "SuperLeft",
    "menu": "ContextMenu",
    "minus": "Minus",
    "num.lock": "NumLock",
    "page.down": "PageDown",
    "page.up": "PageUp",
    "pause": "Pause",
    "period": "Period",
    "print.screen": "PrintScreen",
    "right": "ArrowRight",
    "right.alt": "AltRight",
    "right.bracket": "BracketRight",
    "right.control": "ControlRight",
    "right.shift": "ShiftRight",
    "right.win": "SuperRight",
    "scroll.lock": "ScrollLock",
    "semicolon": "Semicolon",
    "slash": "Slash",
    "space": "Space",
    "tab": "Tab",
    "up": "ArrowUp",
}

#: Names winit has no physical key for, so no binding can ever resolve to them.
UNMAPPABLE_KEYBOARD_NAMES = {"unknown", "world.1", "world.2"}

WINIT_MOUSE_BUTTONS = {
    "left": "Left",
    "right": "Right",
    "middle": "Middle",
}


def _winit_key_code(suffix: str) -> str | None:
    if suffix in UNMAPPABLE_KEYBOARD_NAMES:
        return None
    if len(suffix) == 1 and suffix.isalpha():
        return f"Key{suffix.upper()}"
    if len(suffix) == 1 and suffix.isdigit():
        return f"Digit{suffix}"
    if suffix.startswith("f") and suffix[1:].isdigit():
        return f"F{suffix[1:]}"
    if suffix.startswith("keypad.") and suffix[len("keypad.") :].isdigit():
        return f"Numpad{suffix[len('keypad.') :]}"
    return WINIT_KEY_CODES.get(suffix)


def winit_bindings(keys):
    """`(glfw value, winit ident)` for every named key winit can deliver."""

    keyboard: list[tuple[int, str]] = []
    mouse: list[tuple[int, str]] = []
    for key in keys:
        if key.key_type == "KEYSYM":
            suffix = key.name.removeprefix("key.keyboard.")
            if suffix == key.name:
                raise SystemExit(f"unexpected keysym name: {key.name}")
            code = _winit_key_code(suffix)
            if code is None:
                if suffix not in UNMAPPABLE_KEYBOARD_NAMES:
                    raise SystemExit(f"no winit KeyCode for {key.name}")
                continue
            keyboard.append((key.value, code))
        elif key.key_type == "MOUSE":
            suffix = key.name.removeprefix("key.mouse.")
            button = WINIT_MOUSE_BUTTONS.get(suffix)
            if button is not None:
                mouse.append((key.value, button))
        else:
            raise SystemExit(f"unexpected key type for {key.name}: {key.key_type}")
    return keyboard, mouse


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, default=DEFAULT_SOURCE_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--rust-output", type=Path, default=DEFAULT_RUST_OUTPUT)
    parser.add_argument("--check", action="store_true")
    return parser.parse_args()


def catalog_entries(source_root: Path):
    raw = (source_root / INPUT_CONSTANTS_SOURCE).read_bytes()
    keys = named_keys(parse_source(INPUT_CONSTANTS_SOURCE, raw))
    raw = (source_root / OPTIONS_SOURCE).read_bytes()
    mappings = key_mappings(parse_source(OPTIONS_SOURCE, raw))
    known = {(key.key_type, key.value) for key in keys}
    for mapping in mappings:
        if (mapping.default_type, mapping.default_value) not in known:
            raise SystemExit(
                f"{mapping.name} defaults to an unnamed key: "
                f"{mapping.default_type} {mapping.default_value}"
            )
    return keys, mappings


def render_rust_catalog(keys, mappings) -> str:
    lines = [
        "// @generated by scripts/generate-key-mapping-catalog.py; do not edit manually.",
        "// Source: Vanilla 26.1 InputConstants.Type's static initializer and the",
        "// KeyMapping fields Options declares, which together decide how a",
        "// key_<name> line in options.txt reads back.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-key-mapping-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-key-mapping-catalog.py --check",
        "",
        "use winit::{event::MouseButton, keyboard::KeyCode};",
        "",
        "/// The `InputConstants.Type` variants any catalogued key uses, which",
        "/// decide whether a bound value is a GLFW keysym or a mouse button.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub(crate) enum VanillaKeyType {",
    ]
    used = list(
        dict.fromkeys(
            [RUST_KEY_TYPES[key.key_type] for key in keys]
            + [RUST_KEY_TYPES[mapping.default_type] for mapping in mappings]
        )
    )
    lines.extend(f"    {variant}," for variant in used)
    lines.extend(
        [
            "}",
            "",
            "/// One key `InputConstants` names, so `KeyMapping.saveString` and the",
            "/// matching `options.txt` line agree on its text.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(crate) struct VanillaNamedKey {",
            "    pub(crate) name: &'static str,",
            "    pub(crate) key_type: VanillaKeyType,",
            "    pub(crate) value: i32,",
            "}",
            "",
            "/// One `KeyMapping` `Options` declares, in declaration order.",
            "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
            "pub(crate) struct VanillaKeyMapping {",
            "    pub(crate) name: &'static str,",
            "    pub(crate) default_type: VanillaKeyType,",
            "    pub(crate) default_value: i32,",
            "    pub(crate) category: &'static str,",
            "    /// `ToggleKeyMapping`, whose down state follows a toggle option.",
            "    pub(crate) toggle: bool,",
            "    /// The `order` constructor argument the key-binds screen sorts on.",
            "    pub(crate) sort_order: i32,",
            "}",
            "",
            f"pub(crate) const VANILLA_NAMED_KEYS: [VanillaNamedKey; {len(keys)}] = [",
        ]
    )
    for key in keys:
        lines.extend(
            [
                "    VanillaNamedKey {",
                f'        name: "{key.name}",',
                f"        key_type: VanillaKeyType::{RUST_KEY_TYPES[key.key_type]},",
                f"        value: {key.value},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            f"pub(crate) const VANILLA_KEY_MAPPINGS: [VanillaKeyMapping; {len(mappings)}] = [",
        ]
    )
    for mapping in mappings:
        lines.extend(
            [
                "    VanillaKeyMapping {",
                f'        name: "{mapping.name}",',
                f"        default_type: VanillaKeyType::{RUST_KEY_TYPES[mapping.default_type]},",
                f"        default_value: {mapping.default_value},",
                f'        category: "{mapping.category}",',
                f"        toggle: {'true' if mapping.toggle else 'false'},",
                f"        sort_order: {mapping.sort_order},",
                "    },",
            ]
        )
    keyboard, mouse = winit_bindings(keys)
    lines.extend(
        [
            "];",
            "",
            "/// The physical key winit reports for each named GLFW keysym.",
            f"pub(crate) const VANILLA_KEYSYM_KEY_CODES: [(i32, KeyCode); {len(keyboard)}] = [",
        ]
    )
    lines.extend(f"    ({value}, KeyCode::{code})," for value, code in keyboard)
    lines.extend(
        [
            "];",
            "",
            "/// The button winit reports for each named GLFW mouse button.",
            f"pub(crate) const VANILLA_MOUSE_BUTTONS: [(i32, MouseButton); {len(mouse)}] = [",
        ]
    )
    lines.extend(f"    ({value}, MouseButton::{button})," for value, button in mouse)
    lines.append("];")
    lines.append("")
    return "\n".join(lines)


def render_catalog(keys, mappings) -> str:
    document = {
        "generated_by": "scripts/generate-key-mapping-catalog.py",
        "vanilla_version": "26.1",
        "source_files": [INPUT_CONSTANTS_SOURCE, OPTIONS_SOURCE],
        "line_format": "key_<binding name>:<named key>, written by Options.save",
        "named_keys": [
            {"name": key.name, "type": key.key_type, "value": key.value}
            for key in keys
        ],
        "winit_keysym_key_codes": [
            {"value": value, "key_code": code}
            for value, code in winit_bindings(keys)[0]
        ],
        "winit_mouse_buttons": [
            {"value": value, "button": button}
            for value, button in winit_bindings(keys)[1]
        ],
        "key_mappings": [
            {
                "order": mapping.order,
                "field": mapping.field,
                "name": mapping.name,
                "default_type": mapping.default_type,
                "default_value": mapping.default_value,
                "category": mapping.category,
                "toggle": mapping.toggle,
                "sort_order": mapping.sort_order,
            }
            for mapping in mappings
        ],
    }
    return json.dumps(document, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    args = parse_args()
    keys, mappings = catalog_entries(args.source_root)
    outputs = [
        (args.output, render_catalog(keys, mappings)),
        (args.rust_output, render_rust_catalog(keys, mappings)),
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
        print("key mapping catalog: checked output is current")
        return 0
    for path, rendered in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(rendered, encoding="utf-8")
        print(f"key mapping catalog: wrote {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
