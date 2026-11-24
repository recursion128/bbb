#!/usr/bin/env python3
"""Generate compact binary + Rust tables from item_prototype_components_26_1.json."""

from __future__ import annotations

import argparse
import difflib
import json
import math
import os
import struct
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "crates/bbb-facts/data/item_prototype_components_26_1.json"
OUTPUT = ROOT / "crates/bbb-facts/src/item_prototype_components/generated_26_1.rs"
BIN_OUTPUT = ROOT / "crates/bbb-facts/data/item_prototype_components_26_1.bin"

EXPECTED_SOURCE = (
    "BuiltInRegistries.DATA_COMPONENT_INITIALIZERS with "
    "VanillaRegistries.createLookup and each persistent component codec"
)
EXPECTED_ITEM_COUNT = 1506
EXPECTED_COMPONENT_COUNT = 59
EXPECTED_MEMBERSHIP_COUNT = 18981


def float_bits(value: float) -> int:
    if not math.isfinite(value):
        raise ValueError(f"non-finite float in item prototype catalog: {value!r}")
    return struct.unpack(">Q", struct.pack(">d", value))[0]


def build_tables(artifact: dict[str, Any]) -> tuple[
    list[str],
    list[int],
    list[tuple[int, int]],
    list[int],
    list[tuple[int, int]],
    list[int],
    list[tuple[int, int]],
]:
    if artifact.get("vanilla_version") != "26.1":
        raise ValueError("item prototype artifact version must be 26.1")
    if artifact.get("source") != EXPECTED_SOURCE:
        raise ValueError("item prototype artifact source mismatch")
    item_ids = artifact.get("item_resource_ids", [])
    if len(item_ids) != EXPECTED_ITEM_COUNT:
        raise ValueError(f"expected {EXPECTED_ITEM_COUNT} items, got {len(item_ids)}")
    components = artifact.get("component_values", [])
    if len(components) != EXPECTED_COMPONENT_COUNT:
        raise ValueError(
            f"expected {EXPECTED_COMPONENT_COUNT} components, got {len(components)}"
        )

    strings: list[str] = []
    string_index: dict[str, int] = {}
    floats: list[int] = []
    float_index: dict[int, int] = {}
    nodes: list[tuple[int, int]] = []
    node_index: dict[tuple[Any, ...], int] = {}
    array_items: list[int] = []
    object_fields: list[tuple[int, int]] = []

    def intern_str(text: str) -> int:
        idx = string_index.get(text)
        if idx is None:
            idx = len(strings)
            if idx > 0xFFFF:
                raise ValueError("string pool exceeds u16")
            strings.append(text)
            string_index[text] = idx
        return idx

    def intern_float(bits: int) -> int:
        idx = float_index.get(bits)
        if idx is None:
            idx = len(floats)
            if idx > 0xFFFF:
                raise ValueError("float pool exceeds u16")
            floats.append(bits)
            float_index[bits] = idx
        return idx

    def intern_node(value: Any) -> int:
        if isinstance(value, bool):
            key: tuple[Any, ...] = ("b", value)
            encoded = (0, int(value))
        elif isinstance(value, int):
            if not 0 <= value <= 0xFFFF_FFFF:
                raise ValueError(f"int outside u32 range: {value}")
            key = ("u", value)
            encoded = (1, value)
        elif isinstance(value, float):
            bits = float_bits(value)
            fidx = intern_float(bits)
            key = ("f", bits)
            encoded = (2, fidx)
        elif isinstance(value, str):
            sidx = intern_str(value)
            key = ("s", sidx)
            encoded = (3, sidx)
        elif isinstance(value, list):
            children = tuple(intern_node(child) for child in value)
            key = ("a", children)
            if key in node_index:
                return node_index[key]
            offset = len(array_items)
            length = len(children)
            if offset > 0xFFFF or length > 0xFFFF:
                raise ValueError("array pool exceeds u16")
            array_items.extend(children)
            encoded = (4, offset | (length << 16))
        elif isinstance(value, dict):
            # Preserve exact sorted key order matching serde_json::Map (BTreeMap).
            pairs = tuple((intern_str(k), intern_node(v)) for k, v in sorted(value.items()))
            key = ("o", pairs)
            if key in node_index:
                return node_index[key]
            offset = len(object_fields)
            length = len(pairs)
            if offset > 0xFFFF or length > 0xFFFF:
                raise ValueError("object pool exceeds u16")
            object_fields.extend(pairs)
            encoded = (5, offset | (length << 16))
        else:
            raise ValueError(f"unexpected JSON value type: {type(value)!r}")

        idx = node_index.get(key)
        if idx is None:
            idx = len(nodes)
            if idx > 0xFFFF:
                raise ValueError("node pool exceeds u16")
            nodes.append(encoded)
            node_index[key] = idx
        return idx

    by_item: list[list[tuple[int, int]]] = [[] for _ in range(EXPECTED_ITEM_COUNT)]
    prev_type_id = -1
    for component in components:
        type_id = component["type_id"]
        if not (prev_type_id < type_id <= 0xFF):
            raise ValueError(f"invalid or unordered component type_id: {type_id}")
        prev_type_id = type_id
        for group in component["values"]:
            v_idx = intern_node(group["value"])
            prev_end = -1
            for start, end in group["item_id_ranges"]:
                if start < 0 or start > end or end >= EXPECTED_ITEM_COUNT or start <= prev_end:
                    raise ValueError(f"invalid item_id_range [{start}, {end}] for {type_id}")
                prev_end = end
                for item_id in range(start, end + 1):
                    by_item[item_id].append((type_id, v_idx))

    item_offsets = [0]
    item_components: list[tuple[int, int]] = []
    for entries in by_item:
        item_components.extend(entries)
        item_offsets.append(len(item_components))

    if len(item_components) != EXPECTED_MEMBERSHIP_COUNT:
        raise ValueError(
            f"expected {EXPECTED_MEMBERSHIP_COUNT} memberships, got {len(item_components)}"
        )

    return strings, floats, nodes, array_items, object_fields, item_offsets, item_components


def _pack_all(artifact: dict[str, Any]) -> tuple[bytes, dict[str, int]]:
    (
        strings,
        floats,
        nodes,
        array_items,
        object_fields,
        item_offsets,
        item_components,
    ) = build_tables(artifact)

    str_bytes = bytearray()
    str_spans: list[tuple[int, int]] = []
    for text in strings:
        encoded = text.encode("utf-8")
        if len(encoded) > 0xFFFF:
            raise ValueError("string byte length exceeds u16")
        str_spans.append((len(str_bytes), len(encoded)))
        str_bytes.extend(encoded)

    out = bytearray(str_bytes)
    string_spans_offset = len(out)
    for start, length in str_spans:
        out.extend(struct.pack("<IH", start, length))
    floats_offset = len(out)
    for bits in floats:
        out.extend(struct.pack("<Q", bits))
    array_items_offset = len(out)
    for item in array_items:
        out.extend(struct.pack("<H", item))
    object_fields_offset = len(out)
    for key_idx, child_idx in object_fields:
        out.extend(struct.pack("<HH", key_idx, child_idx))
    nodes_offset = len(out)
    for tag, payload in nodes:
        out.extend(struct.pack("<BI", tag, payload))
    item_offsets_offset = len(out)
    for offset in item_offsets:
        out.extend(struct.pack("<H", offset))
    item_components_offset = len(out)
    for type_id, node_idx in item_components:
        out.extend(struct.pack("<BH", type_id, node_idx))

    meta = {
        "node_count": len(nodes),
        "string_spans_offset": string_spans_offset,
        "floats_offset": floats_offset,
        "array_items_offset": array_items_offset,
        "object_fields_offset": object_fields_offset,
        "nodes_offset": nodes_offset,
        "item_offsets_offset": item_offsets_offset,
        "item_components_offset": item_components_offset,
    }
    return bytes(out), meta


def pack_binary(artifact: dict[str, Any]) -> bytes:
    return _pack_all(artifact)[0]


def render(
    artifact: dict[str, Any],
    bin_rel_path: str = "../../data/item_prototype_components_26_1.bin",
) -> str:
    _, meta = _pack_all(artifact)
    lines = [
        "// @generated by scripts/generate-item-prototype-components-table.py;"
        " do not edit manually.",
        "// Source: crates/bbb-facts/data/item_prototype_components_26_1.json.",
        "// Update: cargo dev python scripts/generate-item-prototype-components-table.py",
        "// Check: cargo dev python scripts/generate-item-prototype-components-table.py --check",
        "",
        "use super::StaticProtoNode;",
        "",
        f'const BYTES: &[u8] = include_bytes!("{bin_rel_path}");',
        f"pub(super) const NODE_COUNT: usize = {meta['node_count']};",
        f"const STRING_SPANS_OFFSET: usize = {meta['string_spans_offset']};",
        f"const FLOATS_OFFSET: usize = {meta['floats_offset']};",
        f"const ARRAY_ITEMS_OFFSET: usize = {meta['array_items_offset']};",
        f"const OBJECT_FIELDS_OFFSET: usize = {meta['object_fields_offset']};",
        f"const NODES_OFFSET: usize = {meta['nodes_offset']};",
        f"const ITEM_OFFSETS_OFFSET: usize = {meta['item_offsets_offset']};",
        f"const ITEM_COMPONENTS_OFFSET: usize = {meta['item_components_offset']};",
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
        "pub(super) fn str_at(index: u16) -> &'static str {",
        "    let span = STRING_SPANS_OFFSET + usize::from(index) * 6;",
        "    let start = u32_at(span) as usize;",
        "    let len = usize::from(u16_at(span + 4));",
        "    core::str::from_utf8(&BYTES[start..start + len])",
        '        .expect("valid UTF-8 in item_prototype_components_26_1.bin")',
        "}",
        "",
        "#[inline]",
        "pub(super) fn float_bits_at(index: u16) -> u64 {",
        "    let offset = FLOATS_OFFSET + usize::from(index) * 8;",
        "    u64::from_le_bytes([",
        "        BYTES[offset],",
        "        BYTES[offset + 1],",
        "        BYTES[offset + 2],",
        "        BYTES[offset + 3],",
        "        BYTES[offset + 4],",
        "        BYTES[offset + 5],",
        "        BYTES[offset + 6],",
        "        BYTES[offset + 7],",
        "    ])",
        "}",
        "",
        "#[inline]",
        "pub(super) fn array_item_at(index: usize) -> u16 {",
        "    u16_at(ARRAY_ITEMS_OFFSET + index * 2)",
        "}",
        "",
        "#[inline]",
        "pub(super) fn object_field_at(index: usize) -> (u16, u16) {",
        "    let offset = OBJECT_FIELDS_OFFSET + index * 4;",
        "    (u16_at(offset), u16_at(offset + 2))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn node_at(index: usize) -> StaticProtoNode {",
        "    let offset = NODES_OFFSET + index * 5;",
        "    let tag = BYTES[offset];",
        "    let payload = u32_at(offset + 1);",
        "    match tag {",
        "        0 => StaticProtoNode::Bool(payload != 0),",
        "        1 => StaticProtoNode::UInt(payload),",
        "        2 => StaticProtoNode::Float(payload as u16),",
        "        3 => StaticProtoNode::Str(payload as u16),",
        "        4 => StaticProtoNode::Array {",
        "            offset: (payload & 0xFFFF) as u16,",
        "            len: (payload >> 16) as u16,",
        "        },",
        "        5 => StaticProtoNode::Object {",
        "            offset: (payload & 0xFFFF) as u16,",
        "            len: (payload >> 16) as u16,",
        "        },",
        '        _ => panic!("invalid prototype node tag"),',
        "    }",
        "}",
        "",
        "#[inline]",
        "pub(super) fn item_offset_at(index: usize) -> usize {",
        "    usize::from(u16_at(ITEM_OFFSETS_OFFSET + index * 2))",
        "}",
        "",
        "#[inline]",
        "pub(super) fn item_component_at(index: usize) -> (u8, u16) {",
        "    let offset = ITEM_COMPONENTS_OFFSET + index * 3;",
        "    (BYTES[offset], u16_at(offset + 1))",
        "}",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=REPORT)
    parser.add_argument("--output", type=Path, default=OUTPUT)
    parser.add_argument("--bin-output", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    bin_output = args.bin_output
    if bin_output is None:
        bin_output = BIN_OUTPUT if args.output == OUTPUT else args.output.with_suffix(".bin")
    bin_rel = os.path.relpath(bin_output, args.output.parent).replace("\\", "/")

    artifact = json.loads(args.input.read_text(encoding="utf-8"))
    rendered = render(artifact, bin_rel)
    blob = pack_binary(artifact)
    if args.check:
        actual = args.output.read_text(encoding="utf-8") if args.output.is_file() else ""
        actual_bin = bin_output.read_bytes() if bin_output.is_file() else b""
        ok = True
        if actual != rendered:
            print(
                "".join(
                    difflib.unified_diff(
                        actual.splitlines(keepends=True),
                        rendered.splitlines(keepends=True),
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
    args.output.write_text(rendered, encoding="utf-8")
    bin_output.write_bytes(blob)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
