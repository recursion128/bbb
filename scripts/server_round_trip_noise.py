#!/usr/bin/env python3
"""Prove an official server can read and rewrite a BBB Noise FULL chunk."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import sys
from collections.abc import Mapping
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.anvil import (  # noqa: E402
    AnvilError,
    WorldSaveReader,
    chunk_timestamp,
    read_chunk,
)

DIMENSION = "overworld"
# SharedConstants.java:16-38
DATA_VERSION = 4786
MIN_SECTION_Y = -4
# ChunkStatus.java:17-32
FINAL_HEIGHTMAPS = frozenset(
    {
        "OCEAN_FLOOR",
        "WORLD_SURFACE",
        "MOTION_BLOCKING",
        "MOTION_BLOCKING_NO_LEAVES",
    }
)
REQUIRED_TAGS = {
    "sections": list,
    "block_entities": list,
    "block_ticks": list,
    "fluid_ticks": list,
    "PostProcessing": list,
    "Heightmaps": dict,
    "structures": dict,
}
# SerializableChunkData.java:334-477
PROTO_ONLY_TAGS = ("entities", "carving_mask")
STORAGE_NAMES = ("region", "poi", "entities")
TERRAIN_SECTION_YS = frozenset(range(-4, 20))
# LevelLightEngine.java:157-166
LIGHT_PADDING_SECTION_YS = frozenset({-5, 20})
LIGHT_BYTES = 2048


class NoiseResumeError(RuntimeError):
    """The Noise FULL hand-off did not satisfy its frozen contract."""


def _tree_digest(root: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*"), key=lambda item: item.as_posix()):
        if path.is_symlink():
            raise NoiseResumeError(f"world trees may not contain symlinks: {path}")
        relative = path.relative_to(root).as_posix().encode()
        digest.update(b"d\0" if path.is_dir() else b"f\0")
        digest.update(relative)
        digest.update(b"\0")
        if path.is_file():
            with path.open("rb") as handle:
                for block in iter(lambda: handle.read(1024 * 1024), b""):
                    digest.update(block)
    return digest.hexdigest()


def _nested(left: Path, right: Path) -> bool:
    try:
        left.relative_to(right)
    except ValueError:
        return False
    return True


def prepare(input_world: Path, output_world: Path) -> dict[str, Any]:
    source = input_world.resolve()
    target = output_world.resolve()
    if not source.is_dir():
        raise NoiseResumeError(f"input world is not a directory: {source}")
    if target.exists():
        raise NoiseResumeError(f"output world already exists: {target}")
    if _nested(source, target) or _nested(target, source):
        raise NoiseResumeError(
            f"input and output worlds must be disjoint, not nested: {source} and {target}"
        )
    dimension = source / "dimensions" / "minecraft" / DIMENSION
    if not dimension.is_dir():
        raise NoiseResumeError(f"input world has no Overworld dimension directory: {dimension}")

    before = _tree_digest(source)
    shutil.copytree(source, target)
    copied_dimension = target / "dimensions" / "minecraft" / DIMENSION
    for name in STORAGE_NAMES:
        storage = copied_dimension / name
        if storage.exists():
            if not storage.is_dir() or storage.is_symlink():
                raise NoiseResumeError(f"copied {name} storage is not a plain directory: {storage}")
            shutil.rmtree(storage)
        storage.mkdir()
    lock = target / "session.lock"
    if lock.exists():
        if not lock.is_file() or lock.is_symlink():
            raise NoiseResumeError(f"copied session.lock is not a plain file: {lock}")
        lock.unlink()

    leftovers = sorted(
        path.relative_to(target).as_posix()
        for path in target.rglob("*")
        if path.is_file() and path.suffix in {".mca", ".mcc"}
    )
    if leftovers:
        raise NoiseResumeError(
            "prepared output still contains chunk storage outside the cleared Overworld paths: "
            + ", ".join(leftovers)
        )
    after = _tree_digest(source)
    if after != before:
        raise NoiseResumeError(f"input world changed while it was being prepared: {source}")
    return {
        "input_world": str(source),
        "output_world": str(target),
        "input_sha256": before,
        "cleared": [
            str((copied_dimension / name).relative_to(target)) for name in STORAGE_NAMES
        ],
    }


def _load_mapping(path: Path, label: str) -> Mapping[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise NoiseResumeError(f"cannot read {label} at {path}: {error}") from error
    if not isinstance(value, dict):
        raise NoiseResumeError(f"{label} at {path} is not a JSON object")
    return value


def _integer(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise NoiseResumeError(f"{label} must be an integer, got {value!r}")
    return value


def marker_from_probe(path: Path) -> dict[str, int]:
    report = _load_mapping(path, "BBB probe report")
    replay = report.get("interaction_replay")
    if not isinstance(replay, dict):
        raise NoiseResumeError("BBB probe report has no interaction_replay object")
    for field, wanted in (
        ("placed_block_name", "minecraft:stone"),
        ("note_block_name", "minecraft:note_block"),
    ):
        if replay.get(field) != wanted:
            raise NoiseResumeError(
                f"BBB probe interaction_replay.{field} is {replay.get(field)!r}, not {wanted}"
            )
    marker = replay.get("placed_block")
    if not isinstance(marker, dict):
        raise NoiseResumeError("BBB probe interaction_replay has no placed_block coordinates")
    return {axis: _integer(marker.get(axis), f"placed_block.{axis}") for axis in ("x", "y", "z")}


def _state_name(reader: WorldSaveReader, x: int, y: int, z: int) -> str | None:
    state = reader.block_state(x, y, z)
    return None if state is None else state.name


def require_markers(
    reader: WorldSaveReader, marker: Mapping[str, int], *, actor: str, control: bool
) -> None:
    x, y, z = marker["x"], marker["y"], marker["z"]
    wanted = [(y, "minecraft:stone"), (y + 1, "minecraft:note_block")]
    if control:
        wanted.append((y + 2, "minecraft:gold_block"))
    for block_y, block in wanted:
        actual = _state_name(reader, x, block_y, z)
        if actual != block:
            raise NoiseResumeError(
                f"{actor} holds {actual or '<unwritten>'} at ({x}, {block_y}, {z}), not {block}"
            )


def require_full_schema(chunk: Mapping[str, Any], chunk_x: int, chunk_z: int) -> dict[str, Any]:
    expected_scalars = {
        "DataVersion": DATA_VERSION,
        "xPos": chunk_x,
        "zPos": chunk_z,
        "yPos": MIN_SECTION_Y,
        "Status": "minecraft:full",
        "isLightOn": 1,
    }
    for tag, wanted in expected_scalars.items():
        actual = chunk.get(tag)
        if tag != "Status" and isinstance(actual, bool):
            raise NoiseResumeError(f"FULL chunk tag {tag} is boolean {actual!r}, not {wanted!r}")
        if actual != wanted:
            raise NoiseResumeError(f"FULL chunk tag {tag} is {actual!r}, not {wanted!r}")
    for tag, kind in REQUIRED_TAGS.items():
        if tag not in chunk:
            raise NoiseResumeError(f"FULL chunk is missing required {tag} tag")
        if not isinstance(chunk[tag], kind):
            raise NoiseResumeError(
                f"FULL chunk tag {tag} is {type(chunk[tag]).__name__}, not {kind.__name__}"
            )
    if not chunk["sections"]:
        raise NoiseResumeError("FULL chunk sections list is empty")
    section_ys: set[int] = set()
    light_arrays = 0
    for index, section in enumerate(chunk["sections"]):
        if not isinstance(section, dict):
            raise NoiseResumeError(f"FULL chunk sections[{index}] is not a compound")
        section_y = _integer(section.get("Y"), f"FULL chunk sections[{index}].Y")
        if section_y in section_ys:
            raise NoiseResumeError(f"FULL chunk has duplicate section Y {section_y}")
        section_ys.add(section_y)
        if section_y not in TERRAIN_SECTION_YS | LIGHT_PADDING_SECTION_YS:
            raise NoiseResumeError(f"FULL chunk section Y {section_y} is outside -5..20")
        if section_y in TERRAIN_SECTION_YS:
            for tag in ("block_states", "biomes"):
                if not isinstance(section.get(tag), dict):
                    raise NoiseResumeError(
                        f"FULL terrain section Y {section_y} has no {tag} compound"
                    )
        elif "block_states" in section or "biomes" in section:
            raise NoiseResumeError(
                f"FULL light-padding section Y {section_y} carries terrain containers"
            )
        for tag in ("BlockLight", "SkyLight"):
            if tag not in section:
                continue
            values = section[tag]
            if not isinstance(values, list) or len(values) != LIGHT_BYTES or any(
                isinstance(value, bool)
                or not isinstance(value, int)
                or value < -128
                or value > 127
                for value in values
            ):
                raise NoiseResumeError(
                    f"FULL chunk section Y {section_y} {tag} is not exactly {LIGHT_BYTES} bytes"
                )
            light_arrays += 1
        if section_y in LIGHT_PADDING_SECTION_YS and not any(
            tag in section for tag in ("BlockLight", "SkyLight")
        ):
            raise NoiseResumeError(
                f"FULL light-padding section Y {section_y} carries no light array"
            )
    missing_sections = sorted(TERRAIN_SECTION_YS - section_ys)
    if missing_sections:
        raise NoiseResumeError(
            "FULL chunk is missing terrain section Y values: "
            + ", ".join(str(value) for value in missing_sections)
        )
    unexpected = [tag for tag in PROTO_ONLY_TAGS if tag in chunk]
    if unexpected:
        raise NoiseResumeError(
            "LEVELCHUNK contains ProtoChunk-only tag(s): " + ", ".join(unexpected)
        )
    heightmaps = chunk["Heightmaps"]
    if set(heightmaps) != FINAL_HEIGHTMAPS:
        raise NoiseResumeError(
            "FULL chunk Heightmaps keys are "
            f"{sorted(heightmaps)}, not {sorted(FINAL_HEIGHTMAPS)}"
        )
    for name, values in heightmaps.items():
        if not isinstance(values, list) or len(values) != 37 or any(
            isinstance(value, bool) or not isinstance(value, int) for value in values
        ):
            raise NoiseResumeError(f"FULL chunk Heightmaps.{name} is not exactly 37 longs")
    structures = chunk["structures"]
    for name in ("starts", "References"):
        if not isinstance(structures.get(name), dict):
            raise NoiseResumeError(f"FULL chunk structures has no {name} compound")
    return {
        "data_version": chunk["DataVersion"],
        "status": chunk["Status"],
        "y_pos": chunk["yPos"],
        "sections": len(chunk["sections"]),
        "light_arrays": light_arrays,
        "block_entities": len(chunk["block_entities"]),
        "block_ticks": len(chunk["block_ticks"]),
        "fluid_ticks": len(chunk["fluid_ticks"]),
        "post_processing_sections": len(chunk["PostProcessing"]),
        "heightmaps": {name: len(heightmaps[name]) for name in sorted(heightmaps)},
        "structure_keys": sorted(chunk["structures"]),
    }


def inspect(world: Path, probe: Path, *, actor: str) -> dict[str, Any]:
    if not actor.strip():
        raise NoiseResumeError("actor must not be empty")
    marker = marker_from_probe(probe)
    chunk_x, chunk_z = marker["x"] >> 4, marker["z"] >> 4
    reader = WorldSaveReader(world, DIMENSION)
    try:
        chunk = read_chunk(reader.region_dir, chunk_x, chunk_z)
        if chunk is None:
            raise NoiseResumeError(
                f"{actor} wrote no chunk at ({chunk_x}, {chunk_z}) under {reader.region_dir}"
            )
        require_markers(reader, marker, actor=actor, control=False)
        schema = require_full_schema(chunk, chunk_x, chunk_z)
        stamp = chunk_timestamp(reader.region_dir, chunk_x, chunk_z)
    except AnvilError as error:
        raise NoiseResumeError(f"cannot read {actor}'s Noise FULL chunk: {error}") from error
    if stamp is None:
        raise NoiseResumeError(
            f"{actor} wrote no region timestamp for chunk ({chunk_x}, {chunk_z})"
        )
    return {
        "actor": actor,
        "published_world": str(world.resolve()),
        "marker": marker,
        "chunk": {"x": chunk_x, "z": chunk_z},
        "published_at": stamp,
        "schema": schema,
    }


def _published_report(path: Path) -> Mapping[str, Any]:
    report = _load_mapping(path, "inspect report")
    if not isinstance(report.get("published_world"), str):
        raise NoiseResumeError(f"inspect report at {path} has no published_world")
    if not isinstance(report.get("schema"), dict):
        raise NoiseResumeError(f"inspect report at {path} has no schema summary")
    marker = report.get("marker")
    if not isinstance(marker, dict):
        raise NoiseResumeError(f"inspect report at {path} has no marker coordinates")
    parsed_marker = {
        axis: _integer(marker.get(axis), f"inspect marker.{axis}") for axis in ("x", "y", "z")
    }
    chunk = report.get("chunk")
    expected_chunk = {"x": parsed_marker["x"] >> 4, "z": parsed_marker["z"] >> 4}
    if chunk != expected_chunk:
        raise NoiseResumeError(
            f"inspect report chunk is {chunk!r}, not marker chunk {expected_chunk!r}"
        )
    stamp = _integer(report.get("published_at"), "inspect published_at")
    if stamp <= 0:
        raise NoiseResumeError(f"inspect published_at must be positive, got {stamp}")
    return report


def verify(world: Path, published: Path, *, actor: str) -> dict[str, Any]:
    if not actor.strip():
        raise NoiseResumeError("actor must not be empty")
    report = _published_report(published)
    marker = {axis: int(report["marker"][axis]) for axis in ("x", "y", "z")}
    chunk_x, chunk_z = marker["x"] >> 4, marker["z"] >> 4
    reader = WorldSaveReader(world, DIMENSION)
    try:
        chunk = read_chunk(reader.region_dir, chunk_x, chunk_z)
        if chunk is None:
            raise NoiseResumeError(
                f"{actor} wrote no chunk at ({chunk_x}, {chunk_z}) under {reader.region_dir}"
            )
        require_markers(reader, marker, actor=actor, control=True)
        schema = require_full_schema(chunk, chunk_x, chunk_z)
        republished_at = chunk_timestamp(reader.region_dir, chunk_x, chunk_z)
    except AnvilError as error:
        raise NoiseResumeError(f"cannot read {actor}'s Noise FULL chunk: {error}") from error
    published_at = int(report["published_at"])
    if republished_at is None:
        raise NoiseResumeError(
            f"{actor} wrote no region timestamp for chunk ({chunk_x}, {chunk_z})"
        )
    if republished_at <= published_at:
        raise NoiseResumeError(
            f"{actor}'s chunk timestamp {republished_at} did not advance beyond BBB's "
            f"published_at {published_at}"
        )
    return {
        "actor": actor,
        "world": str(world.resolve()),
        "published_world": report["published_world"],
        "marker": marker,
        "chunk": {"x": chunk_x, "z": chunk_z},
        "published_at": published_at,
        "republished_at": republished_at,
        "schema": schema,
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    preparer = commands.add_parser("prepare", help="clear chunk storage in a copied world")
    preparer.add_argument("--input-world", type=Path, required=True)
    preparer.add_argument("--output-world", type=Path, required=True)

    inspector = commands.add_parser("inspect", help="inspect the Noise FULL chunk BBB wrote")
    inspector.add_argument("--world", type=Path, required=True)
    inspector.add_argument("--probe", type=Path, required=True)
    inspector.add_argument("--actor", required=True)

    verifier = commands.add_parser("verify", help="verify the official server rewrote the chunk")
    verifier.add_argument("--world", type=Path, required=True)
    verifier.add_argument("--published", type=Path, required=True)
    verifier.add_argument("--actor", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        if arguments.command == "prepare":
            report = prepare(arguments.input_world, arguments.output_world)
        elif arguments.command == "inspect":
            report = inspect(arguments.world, arguments.probe, actor=arguments.actor)
        else:
            report = verify(arguments.world, arguments.published, actor=arguments.actor)
    except (NoiseResumeError, OSError) as error:
        print(f"round-trip noise: {error}", file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
