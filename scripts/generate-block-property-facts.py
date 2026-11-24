#!/usr/bin/env python3
"""Generate typed Vanilla block properties and their Comparable values."""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
BLOCK_STATES = REPO_ROOT / "crates/bbb-facts/data/block_states_26_1.json"
DEFAULT_OUTPUT = REPO_ROOT / "crates/bbb-facts/src/block_state_types/property_facts_26_1.rs"
DEFAULT_BIN_OUTPUT = REPO_ROOT / "crates/bbb-facts/data/block_property_facts_26_1.bin"
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/BlockPropertyFactsReport.java"
TOKEN = re.compile(r"[a-z0-9_]+")


def expected_properties(report):
    if report.get("version") != "26.1":
        raise ValueError("block-state report version must be 26.1")
    states = report["states"]
    properties = {}
    for expected_id, state in enumerate(states):
        if state["id"] != expected_id:
            raise ValueError("block-state report must be contiguous")
        for name, value in sorted(state.get("properties", {}).items()):
            properties.setdefault((state["name"], name), set()).add(value)
    return len(states), properties


def runtime_facts(args: argparse.Namespace) -> str:
    jars = [args.client_jar, *sorted(args.libraries_root.rglob("*.jar"))]
    missing = [path for path in (args.javac, args.java, *jars[:1]) if not path.is_file()]
    if missing:
        raise FileNotFoundError(
            "missing Vanilla runtime probe input: " + ", ".join(str(path) for path in missing)
        )
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(prefix="bbb-block-property-") as directory:
        classes = Path(directory)
        subprocess.run(
            [
                str(args.javac),
                "-cp",
                classpath,
                "-d",
                str(classes),
                str(PROBE_SOURCE),
            ],
            check=True,
        )
        result = subprocess.run(
            [
                str(args.java),
                "-cp",
                os.pathsep.join((str(classes), classpath)),
                "BlockPropertyFactsReport",
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError("Vanilla block-property probe failed:\n" + result.stderr.strip())
    return result.stdout


def parse_facts(source, state_count, expected):
    lines = source.splitlines()
    header = re.fullmatch(r"version=26\.1 states=(\d+) digits=(\d+)", lines[0]) if lines else None
    if header is None or int(header[1]) != state_count:
        raise ValueError("invalid runtime report header")
    digit_count = int(header[2])
    digits = {}
    rows = {}
    for line in lines[1:]:
        fields = line.split("\t")
        if fields[0] == "digit":
            if len(fields) != 3:
                raise ValueError("invalid digit row")
            code, digit = map(int, fields[1:])
            if (
                code in digits
                or not 0 <= code <= 65535
                or 0xD800 <= code <= 0xDFFF
                or not 0 <= digit <= 9
            ):
                raise ValueError("invalid or duplicate BMP digit")
            digits[code] = digit
            continue
        if len(fields) != 4:
            raise ValueError(f"invalid property row: {line!r}")
        block, name, kind, raw = fields
        key = (block, name)
        if key in rows:
            raise ValueError(f"duplicate property {key}")
        if key not in expected:
            raise ValueError(f"unexpected property {key}")
        if kind not in {"boolean", "integer", "enum"}:
            raise ValueError(f"unknown property kind {kind}")
        values = []
        for pair in raw.split(","):
            parts = pair.split(":")
            if len(parts) != 2 or TOKEN.fullmatch(parts[0]) is None:
                raise ValueError(f"invalid property value {pair!r}")
            token, rank = parts[0], int(parts[1])
            if not 0 <= rank <= 2**31 - 1:
                raise ValueError("property rank outside nonnegative i32")
            values.append((token, rank))
        if len({token for token, _ in values}) != len(values):
            raise ValueError(f"duplicate value for {key}")
        if len({rank for _, rank in values}) != len(values):
            raise ValueError(f"duplicate rank for {key}")
        if {token for token, _ in values} != expected[key]:
            raise ValueError(f"property value domain differs for {key}")
        if kind == "boolean" and dict(values) != {"false": 0, "true": 1}:
            raise ValueError(f"invalid boolean values for {key}")
        if kind == "integer":
            numbers = sorted(rank for _, rank in values)
            if any(token != str(rank) for token, rank in values):
                raise ValueError(f"integer token differs from value for {key}")
            if len(numbers) < 2 or numbers[-1] - numbers[0] + 1 != len(numbers):
                raise ValueError(f"non-contiguous integer domain for {key}")
        rows[key] = (kind, tuple(values))
    if rows.keys() != expected.keys():
        raise ValueError(f"missing properties: {sorted(expected.keys() - rows.keys())}")
    if len(digits) != digit_count or any(digits.get(48 + n) != n for n in range(10)):
        raise ValueError("missing digit facts")
    return {key: rows[key] for key in expected}, digits


def _pack_tables(rows, digits):
    string_bytes = bytearray()
    string_spans: dict[str, tuple[int, int]] = {}

    def intern(text: str) -> tuple[int, int]:
        span = string_spans.get(text)
        if span is None:
            encoded = text.encode("utf-8")
            span = (len(string_bytes), len(encoded))
            string_bytes.extend(encoded)
            string_spans[text] = span
        return span

    enum_pool: list[tuple[int, int, int]] = []
    enum_cache: dict[tuple[tuple[str, int], ...], tuple[int, int]] = {}
    kind_records: list[tuple[int, int, int, int, int, int, int]] = []

    for (block, name), (kind, values) in rows.items():
        b_off, b_len = intern(block)
        p_off, p_len = intern(name)
        if kind == "boolean":
            kind_records.append((b_off, b_len, p_off, p_len, 0, 0, 0))
        elif kind == "integer":
            ranks = [rank for _, rank in values]
            kind_records.append((b_off, b_len, p_off, p_len, 1, min(ranks), max(ranks)))
        else:
            span = enum_cache.get(values)
            if span is None:
                start = len(enum_pool)
                for token, rank in values:
                    t_off, t_len = intern(token)
                    enum_pool.append((t_off, t_len, rank))
                span = (start, len(values))
                enum_cache[values] = span
            kind_records.append((b_off, b_len, p_off, p_len, 2, span[0], span[1]))

    sorted_digits = sorted(digits.items())
    out = bytearray(string_bytes)
    enum_values_offset = len(out)
    for t_off, t_len, rank in enum_pool:
        out.extend(struct.pack("<IHi", t_off, t_len, rank))
    kinds_offset = len(out)
    for b_off, b_len, p_off, p_len, tag, a, b in kind_records:
        out.extend(struct.pack("<IHIHBii", b_off, b_len, p_off, p_len, tag, a, b))
    digits_offset = len(out)
    for code, digit in sorted_digits:
        out.extend(struct.pack("<HB", code, digit))

    return (
        bytes(out),
        len(enum_pool),
        len(kind_records),
        len(sorted_digits),
        enum_values_offset,
        kinds_offset,
        digits_offset,
    )


def pack_binary(rows, digits) -> bytes:
    return _pack_tables(rows, digits)[0]


def render_rust(
    rows,
    digits,
    bin_rel_path: str = "../../data/block_property_facts_26_1.bin",
) -> str:
    (
        _blob,
        enum_values_count,
        kind_count,
        digit_count,
        enum_values_offset,
        kinds_offset,
        digits_offset,
    ) = _pack_tables(rows, digits)
    lines = [
        "// @generated by scripts/generate-block-property-facts.py; do not edit.",
        "// Property.getPossibleValues/getName and Enum.ordinal from Vanilla 26.1.",
        "// Rows share the registry and property order of the mixed-radix state report.",
        "// BMP digits use Character.digit(char, 10) from the prepared Vanilla JVM.",
        "use super::VanillaBlockPropertyKind;",
        "",
        f'const BYTES: &[u8] = include_bytes!("{bin_rel_path}");',
        f"const ENUM_VALUES_COUNT: usize = {enum_values_count};",
        f"const KIND_COUNT: usize = {kind_count};",
        f"const DIGIT_COUNT: usize = {digit_count};",
        f"const ENUM_VALUES_OFFSET: usize = {enum_values_offset};",
        f"const KINDS_OFFSET: usize = {kinds_offset};",
        f"const DIGITS_OFFSET: usize = {digits_offset};",
        "",
        "#[inline]",
        "fn u16_at(offset: usize) -> u16 {",
        "    u16::from_le_bytes([BYTES[offset], BYTES[offset + 1]])",
        "}",
        "",
        "#[inline]",
        "fn u32_at(offset: usize) -> u32 {",
        "    u32::from_le_bytes([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "    ])",
        "}",
        "",
        "#[inline]",
        "fn i32_at(offset: usize) -> i32 {",
        "    i32::from_le_bytes([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "    ])",
        "}",
        "",
        "#[inline]",
        "fn str_at(offset: usize, len: usize) -> &'static str {",
        "    core::str::from_utf8(&BYTES[offset..offset + len])",
        '        .expect("valid UTF-8 in block_property_facts_26_1.bin")',
        "}",
        "",
        "type VanillaBlockPropertyKindEntry = (&'static str, &'static str, VanillaBlockPropertyKind);",
        "",
        "pub(super) fn vanilla_block_property_kinds() -> &'static [VanillaBlockPropertyKindEntry] {",
        "    static ENUM_VALUES: std::sync::OnceLock<Box<[(&'static str, i32)]>> =",
        "        std::sync::OnceLock::new();",
        "    static KINDS: std::sync::OnceLock<Box<[VanillaBlockPropertyKindEntry]>> =",
        "        std::sync::OnceLock::new();",
        "    KINDS.get_or_init(|| {",
        "        let enum_pool: &'static [(&'static str, i32)] = ENUM_VALUES.get_or_init(|| {",
        "            (0..ENUM_VALUES_COUNT)",
        "                .map(|idx| {",
        "                    let base = ENUM_VALUES_OFFSET + idx * 10;",
        "                    (",
        "                        str_at(u32_at(base) as usize, u16_at(base + 4) as usize),",
        "                        i32_at(base + 6),",
        "                    )",
        "                })",
        "                .collect::<Vec<_>>()",
        "                .into_boxed_slice()",
        "        });",
        "        (0..KIND_COUNT)",
        "            .map(|idx| {",
        "                let base = KINDS_OFFSET + idx * 21;",
        "                let block = str_at(u32_at(base) as usize, u16_at(base + 4) as usize);",
        "                let prop = str_at(u32_at(base + 6) as usize, u16_at(base + 10) as usize);",
        "                let tag = BYTES[base + 12];",
        "                let a = i32_at(base + 13);",
        "                let b = i32_at(base + 17);",
        "                let kind = match tag {",
        "                    0 => VanillaBlockPropertyKind::Boolean,",
        "                    1 => VanillaBlockPropertyKind::Integer { min: a, max: b },",
        "                    2 => VanillaBlockPropertyKind::Enum {",
        "                        values: &enum_pool[a as usize..(a + b) as usize],",
        "                    },",
        '                    _ => panic!("invalid property kind tag"),',
        "                };",
        "                (block, prop, kind)",
        "            })",
        "            .collect::<Vec<_>>()",
        "            .into_boxed_slice()",
        "    })",
        "}",
        "",
        "pub(super) fn java_bmp_decimal_digit(code: u16) -> Option<i32> {",
        "    let mut lo = 0;",
        "    let mut hi = DIGIT_COUNT;",
        "    while lo < hi {",
        "        let mid = lo + (hi - lo) / 2;",
        "        let base = DIGITS_OFFSET + mid * 3;",
        "        let candidate = u16_at(base);",
        "        if candidate < code {",
        "            lo = mid + 1;",
        "        } else if candidate > code {",
        "            hi = mid;",
        "        } else {",
        "            return Some(i32::from(BYTES[base + 2]));",
        "        }",
        "    }",
        "    None",
        "}",
        "",
    ]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    add_vanilla_runtime(parser)
    parser.add_argument("--facts-input", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--bin-output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    count, expected = expected_properties(json.loads(BLOCK_STATES.read_text()))
    if args.facts_input:
        source = args.facts_input.read_text()
    else:
        resolve_runtime_inputs(args)
        source = runtime_facts(args)
    rows, digits = parse_facts(source, count, expected)
    bin_output = args.bin_output
    if bin_output is None:
        bin_output = (
            DEFAULT_BIN_OUTPUT if args.output == DEFAULT_OUTPUT else args.output.with_suffix(".bin")
        )
    bin_rel = os.path.relpath(bin_output, args.output.parent).replace("\\", "/")
    output = render_rust(rows, digits, bin_rel)
    blob = pack_binary(rows, digits)
    if args.check:
        actual = args.output.read_text() if args.output.exists() else ""
        actual_bin = bin_output.read_bytes() if bin_output.exists() else b""
        ok = True
        if actual != output:
            print(
                "".join(
                    difflib.unified_diff(
                        actual.splitlines(True),
                        output.splitlines(True),
                        fromfile=str(args.output),
                        tofile="generated",
                    )
                ),
                end="",
            )
            ok = False
        if actual_bin != blob:
            sys.stderr.write(
                f"binary output differs at {bin_output} "
                f"(actual {len(actual_bin)} B vs expected {len(blob)} B)\n"
            )
            ok = False
        return 0 if ok else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    bin_output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(output)
    bin_output.write_bytes(blob)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
