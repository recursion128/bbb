#!/usr/bin/env python3
"""Generate the checked-in vanilla 26.1 built-in server datapack snapshot blob.

Vanilla's built-in server datapack is the ``data/minecraft/`` tree shipped in the
server jar (``ServerPacksSource.createVanillaPackSource``). This generator embeds the
RegistryDataLoader surface, the LEVEL_STEM directory, and the built-in feature packs
as one deterministic gzip blob consumed by ``crates/bbb-server-datapack``.

Blob format (inside the gzip stream, records sorted lexicographically by path bytes,
no header and no footer)::

    [u32 LE path_byte_len][path UTF-8 bytes][u64 LE data_byte_len][data bytes]

Paths are relative to ``data/minecraft/`` and use POSIX separators, e.g.
``dimension_type/overworld.json`` or
``datapacks/trade_rebalance/data/minecraft/dimension_type/...``.

Vanilla 26.1 note: ``RegistryDataLoader.DIMENSION_REGISTRIES`` loads
``Registries.LEVEL_STEM`` from the ``dimension/`` pack directory, but the 26.1 jar
ships no ``data/minecraft/dimension/`` tree at all (verified against both the managed
source tree and ``META-INF/versions/26.1/server-26.1.jar``); vanilla bootstraps its
three level stems in code (``WorldPresets`` / ``WorldDimensions``). ``dimension`` is
therefore declared but optional: it is included when a vanilla tree provides it and
contributes zero records otherwise. Every other declared directory must exist.

Update: scripts/python python scripts/generate-vanilla-datapack-snapshot.py
Check:  scripts/python python scripts/generate-vanilla-datapack-snapshot.py --check
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import sys
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_root, resolve_source_root

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-server-datapack/data/vanilla_datapack_snapshot_26_1.bin.gz"
)
DATA_ROOT = Path("data/minecraft")
GZIP_COMPRESS_LEVEL = 9

# The 26.1 RegistryDataLoader surface (WORLDGEN_REGISTRIES and
# DIMENSION_REGISTRIES directories) plus the tag surface (`tags/`), the loot table
# tree (`loot_table/`), the recipe and advancement trees, the structure template
# tree (`structure/`, raw gzip NBT pass-through) and the built-in feature-pack
# root. Deliberately excludes predicate/, item_modifier/, and function/ — the 26.1
# built-in pack ships none of those trees; later phases extend this list.
REQUIRED_DIRECTORIES = (
    "advancement",
    "banner_pattern",
    "cat_sound_variant",
    "cat_variant",
    "chat_type",
    "chicken_sound_variant",
    "chicken_variant",
    "cow_sound_variant",
    "cow_variant",
    "damage_type",
    "datapacks",
    "dialog",
    "dimension_type",
    "enchantment",
    "enchantment_provider",
    "frog_variant",
    "instrument",
    "jukebox_song",
    "loot_table",
    "painting_variant",
    "pig_sound_variant",
    "pig_variant",
    "recipe",
    "structure",
    "tags",
    "test_environment",
    "test_instance",
    "timeline",
    "trade_set",
    "trial_spawner",
    "trim_material",
    "trim_pattern",
    "villager_trade",
    "wolf_sound_variant",
    "wolf_variant",
    "world_clock",
    "worldgen",
    "zombie_nautilus_variant",
)

# Declared directories that a vanilla tree may legitimately lack. In 26.1 the
# LEVEL_STEM ("dimension") directory is absent from the built-in pack; see the
# module docstring.
OPTIONAL_DIRECTORIES = ("dimension",)

DECLARED_DIRECTORIES = REQUIRED_DIRECTORIES + OPTIONAL_DIRECTORIES

# Top-level directories that must never enter the snapshot at this phase.
FORBIDDEN_DIRECTORIES = frozenset(
    (
        "function",
        "item_modifier",
        "predicate",
    )
)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    add_vanilla_root(parser)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--check",
        action="store_true",
        help="byte-compare a regenerated snapshot against the committed artifact",
    )
    args = parser.parse_args(argv)
    args.source_root = resolve_source_root(args)
    return args


def collect_records(source_root: Path) -> list[tuple[str, bytes]]:
    """Read the declared vanilla datapack directories into (path, bytes) records."""
    data_root = source_root / DATA_ROOT
    if not data_root.is_dir():
        raise ValueError(f"missing vanilla data root: {data_root}")
    for directory in FORBIDDEN_DIRECTORIES & set(DECLARED_DIRECTORIES):
        raise ValueError(f"declared directory is forbidden in this phase: {directory}")
    records: list[tuple[str, bytes]] = []
    for directory in DECLARED_DIRECTORIES:
        declared = data_root / directory
        if not declared.is_dir():
            if directory in OPTIONAL_DIRECTORIES:
                continue
            raise ValueError(
                f"missing declared vanilla datapack directory: {declared} "
                "(update DECLARED_DIRECTORIES if the vanilla surface changed)"
            )
        for path in sorted(declared.rglob("*")):
            if not path.is_file():
                continue
            relative = path.relative_to(data_root).as_posix()
            records.append((relative, path.read_bytes()))
    records.sort(key=lambda record: record[0].encode("utf-8"))
    return records


def encode_records(records: list[tuple[str, bytes]]) -> bytes:
    """Serialize records into the raw blob payload (the gzip-wrapped byte stream)."""
    payload = bytearray()
    for path, data in records:
        path_bytes = path.encode("utf-8")
        payload += len(path_bytes).to_bytes(4, "little")
        payload += path_bytes
        payload += len(data).to_bytes(8, "little")
        payload += data
    return bytes(payload)


def compress_payload(payload: bytes) -> bytes:
    """Wrap the raw payload in a deterministic gzip stream (mtime=0, no filename)."""
    buffer = io.BytesIO()
    with gzip.GzipFile(
        filename="",
        mode="wb",
        fileobj=buffer,
        compresslevel=GZIP_COMPRESS_LEVEL,
        mtime=0,
    ) as stream:
        stream.write(payload)
    return buffer.getvalue()


def build_blob(source_root: Path) -> tuple[list[tuple[str, bytes]], bytes]:
    records = collect_records(source_root)
    return records, compress_payload(encode_records(records))


def decode_records(blob: bytes) -> list[tuple[str, bytes]]:
    """Decode a blob back into records; raises ValueError on framing drift."""
    payload = gzip.decompress(blob)
    records: list[tuple[str, bytes]] = []
    offset = 0
    while offset < len(payload):
        if offset + 4 > len(payload):
            raise ValueError(f"truncated path length at payload offset {offset}")
        path_len = int.from_bytes(payload[offset : offset + 4], "little")
        offset += 4
        if offset + path_len > len(payload):
            raise ValueError(f"truncated path at payload offset {offset}")
        path = payload[offset : offset + path_len].decode("utf-8")
        offset += path_len
        if offset + 8 > len(payload):
            raise ValueError(f"truncated data length at payload offset {offset}")
        data_len = int.from_bytes(payload[offset : offset + 8], "little")
        offset += 8
        if offset + data_len > len(payload):
            raise ValueError(f"truncated data for record {path!r} at offset {offset}")
        records.append((path, payload[offset : offset + data_len]))
        offset += data_len
    return records


def blob_summary(records: list[tuple[str, bytes]], blob: bytes) -> str:
    raw_bytes = len(encode_records(records))
    digest = hashlib.sha256(blob).hexdigest()
    return (
        f"files={len(records)} raw_bytes={raw_bytes} "
        f"compressed_bytes={len(blob)} sha256={digest}"
    )


def diff_summary(expected: bytes, actual: bytes) -> str:
    lines = [
        f"committed sha256={hashlib.sha256(actual).hexdigest()} bytes={len(actual)}",
        f"regenerated sha256={hashlib.sha256(expected).hexdigest()} bytes={len(expected)}",
    ]
    try:
        committed_records = dict(decode_records(actual))
        regenerated_records = dict(decode_records(expected))
    except (OSError, ValueError) as error:
        lines.append(f"committed artifact failed to decode: {error}")
        return "\n".join(lines)
    missing = sorted(set(regenerated_records) - set(committed_records))
    extra = sorted(set(committed_records) - set(regenerated_records))
    changed = sorted(
        path
        for path in set(committed_records) & set(regenerated_records)
        if committed_records[path] != regenerated_records[path]
    )
    for label, paths in (("missing", missing), ("extra", extra), ("changed", changed)):
        lines.append(f"{label} records: {len(paths)}")
        lines.extend(f"  {path}" for path in paths[:10])
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    records, blob = build_blob(args.source_root)
    if args.check:
        committed = args.output.read_bytes() if args.output.exists() else b""
        if committed == blob:
            print(f"ok: {args.output} matches regenerated snapshot")
            print(blob_summary(records, blob))
            return 0
        print(f"drift: {args.output} differs from the regenerated snapshot", file=sys.stderr)
        print(diff_summary(blob, committed), file=sys.stderr)
        return 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(blob)
    print(f"wrote {args.output}")
    print(blob_summary(records, blob))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
