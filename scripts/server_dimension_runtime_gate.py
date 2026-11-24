#!/usr/bin/env python3
"""Run and deterministically replay the Java 26.1 dimension runtime gate."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
import time
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

sys.dont_write_bytecode = True

REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = REPO_ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from bbb_tools import anvil  # noqa: E402
from bbb_tools.execution.process import free_port  # noqa: E402

EXPECTED_SEQUENCE = (
    "minecraft:overworld",
    "minecraft:the_nether",
    "minecraft:the_end",
    "minecraft:overworld",
)

RESOURCE_ID_RE = re.compile(r"[a-z0-9_.-]+:[a-z0-9_./-]+")
REGION_RE = re.compile(r"r\.(-?\d+)\.(-?\d+)\.mca")
SHA256_RE = re.compile(r"[0-9a-f]{64}")
# ServerPlayer.java:1102-1129
# ChunkMap.java:1037-1048, ChunkMap.java:1113-1122
# PlayerList.java:644-656
# PlayerChunkSender.java:50-78
MILESTONES = [
    "respawn",
    "player_position",
    "set_chunk_cache_center",
    "level_chunks_load_start",
    "chunk_batch_start",
    "first_chunk",
    "target_chunk",
    "chunk_batch_finished",
]
PROBE_USERNAME = "server-operator"


class DimensionGateError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise DimensionGateError(message)


def atomic_write(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f"{path.name}.tmp.{os.getpid()}")
    try:
        temporary.write_bytes(payload)
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def strict_json(path: Path) -> dict[str, Any]:
    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in items:
            if key in value:
                raise DimensionGateError(f"{path} repeats JSON key {key!r}")
            value[key] = item
        return value

    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=pairs)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise DimensionGateError(f"cannot read {path}: {error}") from error
    if not isinstance(value, dict):
        raise DimensionGateError(f"{path} must contain a JSON object")
    return value


def canonical_digest(value: Any) -> str:
    encoded = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(encoded.encode("utf-8")).hexdigest()


def split_resource_id(value: Any, *, context: str) -> tuple[str, str]:
    if not isinstance(value, str) or RESOURCE_ID_RE.fullmatch(value) is None:
        raise DimensionGateError(f"{context} is not a canonical resource id: {value!r}")
    namespace, path = value.split(":", 1)
    # Identifier.java:230-241
    if namespace == "..":
        raise DimensionGateError(f"{context} has an unsafe resource namespace: {value!r}")
    if any(part in ("", ".", "..") for part in path.split("/")):
        raise DimensionGateError(f"{context} has an unsafe resource path: {value!r}")
    return namespace, path


def dimension_root(world: Path, key: str) -> Path:
    # DimensionType.java:115-117
    namespace, path = split_resource_id(key, context="dimension key")
    root = world.resolve()
    candidate = root / "dimensions" / namespace / Path(*path.split("/"))
    if not candidate.resolve(strict=False).is_relative_to(root):
        raise DimensionGateError(f"dimension key escapes the world: {key!r}")
    return candidate


def _resource_order(value: str) -> tuple[str, str]:
    namespace, path = split_resource_id(value, context="catalog resource id")
    return path, namespace


def validate_configuration_catalog(value: Any, *, side: str) -> dict[str, Any]:
    # RegistryDataLoader.java:133-162
    # ResourceManagerRegistryLoadTask.java:60-67
    require(isinstance(value, Mapping), f"{side} has no configuration_catalog")
    require(
        set(value) == {"selected_known_packs", "packet_order", "registries", "tags"},
        f"{side} configuration_catalog fields drifted",
    )
    # SynchronizeRegistriesTask.java:31-50
    # RegistrySynchronization.java:47-65
    require(
        type(value["selected_known_packs"]) is int and value["selected_known_packs"] == 0,
        f"{side} selected known packs instead of full NBT",
    )
    registries = value["registries"]
    require(
        isinstance(registries, list) and len(registries) == 28,
        f"{side} did not publish 28 registries",
    )
    registry_names: list[str] = []
    for registry_index, registry in enumerate(registries):
        context = f"{side}.registries[{registry_index}]"
        require(
            isinstance(registry, Mapping) and set(registry) == {"registry", "entries"},
            f"{context} fields drifted",
        )
        name = registry["registry"]
        split_resource_id(name, context=f"{context}.registry")
        registry_names.append(name)
        entries = registry["entries"]
        require(isinstance(entries, list) and entries, f"{context}.entries is empty")
        ids: list[str] = []
        for entry_index, entry in enumerate(entries):
            entry_context = f"{context}.entries[{entry_index}]"
            require(
                isinstance(entry, Mapping)
                and set(entry) == {"resource_id", "canonical_nbt_sha256"},
                f"{entry_context} fields drifted",
            )
            resource_id = entry["resource_id"]
            split_resource_id(resource_id, context=f"{entry_context}.resource_id")
            require(
                isinstance(entry["canonical_nbt_sha256"], str)
                and SHA256_RE.fullmatch(entry["canonical_nbt_sha256"]) is not None,
                f"{entry_context} has no canonical NBT digest",
            )
            ids.append(resource_id)
        require(len(ids) == len(set(ids)), f"{context} repeats entries")
        require(
            ids == sorted(ids, key=_resource_order),
            f"{context} entries are not in Vanilla path/namespace order",
        )
    require(len(registry_names) == len(set(registry_names)), f"{side} repeats registry packets")
    require(
        sum(len(registry["entries"]) for registry in registries) > 0,
        f"{side} configuration registry catalog is empty",
    )
    expected_order = [
        "select_known_packs",
        *[f"registry_data:{name}" for name in registry_names],
        "update_tags",
        "finish",
    ]
    require(value["packet_order"] == expected_order, f"{side} configuration packet order drifted")
    tags = value["tags"]
    require(
        isinstance(tags, list) and len(tags) == 15,
        f"{side}.tags does not contain the 15 network tag registries",
    )
    tag_registries: list[str] = []
    for registry_index, registry in enumerate(tags):
        context = f"{side}.tags[{registry_index}]"
        require(
            isinstance(registry, Mapping) and set(registry) == {"registry", "tags"},
            f"{context} fields drifted",
        )
        split_resource_id(registry["registry"], context=f"{context}.registry")
        tag_registries.append(registry["registry"])
        require(
            isinstance(registry["tags"], list) and registry["tags"],
            f"{context}.tags is empty",
        )
        names: list[str] = []
        for tag_index, tag in enumerate(registry["tags"]):
            tag_context = f"{context}.tags[{tag_index}]"
            require(
                isinstance(tag, Mapping) and set(tag) == {"tag", "entries"},
                f"{tag_context} fields drifted",
            )
            split_resource_id(tag["tag"], context=f"{tag_context}.tag")
            require(isinstance(tag["entries"], list), f"{tag_context}.entries is not a list")
            for entry in tag["entries"]:
                split_resource_id(entry, context=f"{tag_context}.entries")
            require(
                len(tag["entries"]) == len(set(tag["entries"])),
                f"{tag_context} repeats holder entries",
            )
            names.append(tag["tag"])
        require(len(names) == len(set(names)), f"{context} repeats tags")
    require(len(tag_registries) == len(set(tag_registries)), f"{side} repeats tag registries")
    return dict(value)


def _level(value: Any, *, context: str, dimension: str) -> dict[str, Any]:
    require(isinstance(value, Mapping), f"{context} is missing")
    required = {
        "dimension",
        "dimension_type_id",
        "dimension_type_name",
        "dimension_type_logical_height",
        "dimension_type_infiniburn_tag",
        "sea_level",
        "min_y",
        "height",
        "is_debug",
        "is_flat",
    }
    require(set(value) == required, f"{context} fields drifted")
    require(
        value["dimension"] == dimension, f"{context} names {value['dimension']!r}, not {dimension}"
    )
    split_resource_id(value["dimension"], context=f"{context}.dimension")
    require(
        type(value["dimension_type_id"]) is int, f"{context}.dimension_type_id is not an integer"
    )
    split_resource_id(value["dimension_type_name"], context=f"{context}.dimension_type_name")
    require(
        type(value["height"]) is int and value["height"] > 0 and value["height"] % 16 == 0,
        f"{context}.height is invalid",
    )
    require(
        type(value["dimension_type_logical_height"]) is int
        and 0 < value["dimension_type_logical_height"] <= value["height"],
        f"{context}.dimension_type_logical_height is invalid",
    )
    infiniburn = value["dimension_type_infiniburn_tag"]
    split_resource_id(infiniburn, context=f"{context}.dimension_type_infiniburn_tag")
    require(type(value["sea_level"]) is int, f"{context}.sea_level is not an integer")
    require(type(value["min_y"]) is int, f"{context}.min_y is not an integer")
    require(value["min_y"] % 16 == 0, f"{context}.min_y is not section aligned")
    require(value["is_debug"] is False, f"{context} unexpectedly reports a debug world")
    require(type(value["is_flat"]) is bool, f"{context}.is_flat is not a bool")
    return dict(value)


def validate_dimension_transitions(value: Any, *, side: str) -> dict[str, Any]:
    require(
        isinstance(value, Mapping) and set(value) == {"initial", "legs"},
        f"{side} has malformed dimension_transitions",
    )
    initial = _level(value["initial"], context=f"{side}.initial", dimension="minecraft:overworld")
    legs = value["legs"]
    targets = list(EXPECTED_SEQUENCE[1:])
    require(
        isinstance(legs, list) and len(legs) == len(targets),
        f"{side} has the wrong transition leg count",
    )
    normalized_legs: list[dict[str, Any]] = []
    for index, (leg, target) in enumerate(zip(legs, targets, strict=True)):
        context = f"{side}.legs[{index}]"
        required = {
            "target_dimension",
            "respawn_count_delta",
            "data_to_keep",
            "level",
            "milestone_order",
            "player_position",
            "target_chunk",
            "first_chunk",
            "target_chunk_completed",
        }
        require(isinstance(leg, Mapping) and set(leg) == required, f"{context} fields drifted")
        require(
            leg["target_dimension"] == target,
            f"{context} targets {leg['target_dimension']!r}, not {target}",
        )
        require(
            type(leg["respawn_count_delta"]) is int and leg["respawn_count_delta"] == 1,
            f"{context} did not receive exactly one Respawn",
        )
        require(
            type(leg["data_to_keep"]) is int and leg["data_to_keep"] == 3,
            f"{context} Respawn did not carry KEEP_ALL_DATA",
        )
        require(leg["milestone_order"] == MILESTONES, f"{context} packet/chunk milestones drifted")
        require(leg["target_chunk_completed"] is True, f"{context} target chunk did not complete")
        position = leg["player_position"]
        require(
            isinstance(position, list)
            and len(position) == 3
            and all(isinstance(coordinate, float) for coordinate in position)
            # `Vec3Argument.vec3()` center-corrects (Vec3Argument.java:30-32), so
            # the whole `0 80 0` each leg teleports to lands on the block centre
            # in x and z (WorldCoordinate.java:34-36).
            and position == [0.5, 80.0, 0.5],
            f"{context}.player_position drifted",
        )
        for name in ("target_chunk", "first_chunk"):
            chunk = leg[name]
            require(
                isinstance(chunk, Mapping)
                and set(chunk) == {"x", "z"}
                and all(type(chunk[axis]) is int for axis in ("x", "z")),
                f"{context}.{name} is malformed",
            )
        position_chunk = {
            "x": math.floor(position[0]) >> 4,
            "z": math.floor(position[2]) >> 4,
        }
        require(
            leg["target_chunk"] == position_chunk,
            f"{context}.player_position is outside the cache-center target chunk",
        )
        normalized = dict(leg)
        normalized["level"] = _level(leg["level"], context=f"{context}.level", dimension=target)
        normalized_legs.append(normalized)
    return {"initial": initial, "legs": normalized_legs}


def compare_probes(bbb: Mapping[str, Any], official: Mapping[str, Any]) -> dict[str, Any]:
    bbb_configuration = validate_configuration_catalog(bbb.get("configuration_catalog"), side="bbb")
    official_configuration = validate_configuration_catalog(
        official.get("configuration_catalog"), side="official"
    )
    require(
        bbb_configuration == official_configuration,
        "BBB and official configuration catalogs differ",
    )
    bbb_transitions = validate_dimension_transitions(bbb.get("dimension_transitions"), side="bbb")
    official_transitions = validate_dimension_transitions(
        official.get("dimension_transitions"), side="official"
    )
    require(
        bbb_transitions == official_transitions, "BBB and official dimension transitions differ"
    )
    return {
        "configuration": {
            "registry_count": len(bbb_configuration["registries"]),
            "registry_catalog_sha256": canonical_digest(bbb_configuration["registries"]),
            "tag_catalog_sha256": canonical_digest(bbb_configuration["tags"]),
            "bbb_equals_official": True,
        },
        "transitions": {
            "sequence": list(EXPECTED_SEQUENCE),
            "catalog_sha256": canonical_digest(bbb_transitions),
            "bbb_equals_official": True,
        },
    }


def marker_plan(probe: Mapping[str, Any]) -> list[dict[str, Any]]:
    transitions = validate_dimension_transitions(probe.get("dimension_transitions"), side="probe")
    levels = [transitions["initial"], *[leg["level"] for leg in transitions["legs"]]]
    plan: list[dict[str, Any]] = []
    for level in levels:
        if any(item["key"] == level["dimension"] for item in plan):
            continue
        index = len(plan)
        plan.append(
            {
                "key": level["dimension"],
                "min_y": level["min_y"],
                "height": level["height"],
                "x": index * 512 + 8,
                "y": level["min_y"] + level["height"] // 2,
                "z": index * 512 + 8,
            }
        )
    require(
        [item["key"] for item in plan] == list(dict.fromkeys(EXPECTED_SEQUENCE)),
        "probe did not describe all built-in dimensions",
    )
    return plan


def occupied_slots(path: Path) -> list[tuple[int, int]]:
    if path.is_symlink():
        raise DimensionGateError(f"region file is a symlink: {path}")
    match = REGION_RE.fullmatch(path.name)
    if match is None:
        raise DimensionGateError(f"unexpected region filename: {path}")
    size = path.stat().st_size
    if size == 0:
        return []
    if size < anvil.SECTOR_BYTES * 2:
        raise DimensionGateError(f"region file is shorter than its complete header: {path}")
    with path.open("rb") as handle:
        table = handle.read(anvil.SECTOR_BYTES)
    region_x, region_z = (int(value) for value in match.groups())
    occupied: list[tuple[int, int]] = []
    for index in range(1024):
        entry = table[index * 4 : index * 4 + 4]
        sector = int.from_bytes(entry[:3], "big")
        count = entry[3]
        if sector == 0 and count == 0:
            continue
        if (
            sector < 2
            or count == 0
            or sector * anvil.SECTOR_BYTES + 5 > size
            or (sector + count) * anvil.SECTOR_BYTES > size
        ):
            raise DimensionGateError(f"region file has an invalid occupied slot {index}: {path}")
        occupied.append((region_x * 32 + index % 32, region_z * 32 + index // 32))
    return occupied


def _reject_symlinks(root: Path) -> None:
    if root.is_symlink():
        raise DimensionGateError(f"world dimension tree is a symlink: {root}")
    if not root.exists():
        return
    for path in root.rglob("*"):
        if path.is_symlink():
            raise DimensionGateError(f"world dimension tree contains a symlink: {path}")


def occupied_dimension_columns(
    world: Path, keys: Sequence[str]
) -> dict[str, list[tuple[int, int]]]:
    roots = {dimension_root(world, key).resolve(strict=False): key for key in keys}
    dimensions_dir = world / "dimensions"
    _reject_symlinks(dimensions_dir)
    found = {key: [] for key in keys}
    if not dimensions_dir.exists():
        return found
    occupied_by_dir: dict[Path, list[tuple[int, int]]] = {}
    for region_file in sorted(dimensions_dir.rglob("*.mca")):
        columns = occupied_slots(region_file)
        if not columns:
            continue
        region_dir = region_file.parent
        require(
            region_dir.name in {"region", "entities", "poi"},
            f"occupied Anvil file is under an unknown storage root: {region_file}",
        )
        occupied_by_dir.setdefault(region_dir, []).extend(columns)
    for region_dir, columns in occupied_by_dir.items():
        key = roots.get(region_dir.parent.resolve(strict=False))
        if key is None:
            raise DimensionGateError(
                f"occupied region belongs to an undeclared dimension root: {region_dir}"
            )
        if region_dir.name == "region":
            found[key].extend(columns)
    return {key: sorted(set(columns)) for key, columns in found.items()}


def _dimension_fact(
    world: Path, item: Mapping[str, Any], *, previous: Mapping[str, Any] | None = None
) -> dict[str, Any]:
    key = item["key"]
    root = dimension_root(world, key)
    x, y, z = item["x"], item["y"], item["z"]
    chunk_x, chunk_z = x >> 4, z >> 4
    chunk = anvil.read_chunk(root / "region", chunk_x, chunk_z)
    require(chunk is not None, f"{key} marker chunk is absent")
    expected_y_pos = item["min_y"] // 16
    require(
        chunk.get("yPos") == expected_y_pos,
        f"{key} chunk yPos is {chunk.get('yPos')!r}, not {expected_y_pos}",
    )
    sections = chunk.get("sections")
    require(isinstance(sections, list), f"{key} marker chunk has no sections")
    section_by_y: dict[int, Mapping[str, Any]] = {}
    for index, section in enumerate(sections):
        require(isinstance(section, Mapping), f"{key} sections[{index}] is not a compound")
        section_y = section.get("Y")
        require(type(section_y) is int, f"{key} sections[{index}].Y is not an integer")
        require(section_y not in section_by_y, f"{key} repeats section Y {section_y}")
        section_by_y[section_y] = section
    expected_sections = list(range(expected_y_pos, expected_y_pos + item["height"] // 16))
    actual_sections = sorted(section_by_y)
    require(
        set(expected_sections).issubset(section_by_y),
        f"{key} section geometry is missing terrain sections",
    )
    permitted_sections = set(expected_sections) | {
        expected_y_pos - 1,
        expected_y_pos + item["height"] // 16,
    }
    require(
        set(actual_sections).issubset(permitted_sections),
        f"{key} section geometry contains out-of-range sections",
    )
    for section_y in expected_sections:
        section = section_by_y[section_y]
        require(
            isinstance(section.get("block_states"), Mapping)
            and isinstance(section.get("biomes"), Mapping),
            f"{key} terrain section {section_y} has no block-state/biome containers",
        )
    for section_y in set(actual_sections) - set(expected_sections):
        section = section_by_y[section_y]
        require(
            "block_states" not in section
            and "biomes" not in section
            and ("BlockLight" in section or "SkyLight" in section),
            f"{key} padding section {section_y} has an invalid shape",
        )
    diamond = anvil.chunk_block_state(chunk, x, y, z)
    require(
        diamond is not None and diamond.name == "minecraft:diamond_block",
        f"{key} lost its BBB diamond marker",
    )
    # RegionFile.java:285-315
    stamp = anvil.chunk_timestamp(root / "region", chunk_x, chunk_z)
    require(isinstance(stamp, int) and stamp > 0, f"{key} marker chunk has no publish timestamp")
    fact: dict[str, Any] = {
        "key": key,
        "path": root.resolve().relative_to(world.resolve()).as_posix(),
        "marker_chunk": {"x": chunk_x, "z": chunk_z},
        "min_y": item["min_y"],
        "height": item["height"],
        "y_pos": chunk["yPos"],
        "section_y": expected_sections,
        "published_at": stamp if previous is None else previous["published_at"],
    }
    if previous is not None:
        gold = anvil.chunk_block_state(chunk, x + 1, y, z)
        require(
            gold is not None and gold.name == "minecraft:gold_block",
            f"{key} lost its official gold marker",
        )
        require(fact["path"] == previous["path"], f"{key} moved to a different dimension path")
        require(
            fact["y_pos"] == previous["y_pos"] and fact["section_y"] == previous["section_y"],
            f"{key} geometry changed after official reload",
        )
        require(
            stamp > previous["published_at"],
            f"{key} region timestamp did not advance across official flush",
        )
        fact["republished_at"] = stamp
    return fact


def inspect_bbb_client_world(world: Path, plan: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    require(world.is_dir() and not world.is_symlink(), f"BBB world is unavailable: {world}")
    keys = [str(item["key"]) for item in plan]
    require(len(keys) == len(set(keys)), "marker plan repeats a dimension key")
    occupied = occupied_dimension_columns(world, keys)
    written = [key for key in keys if occupied[key]]
    require(
        written == keys, f"written dimensions {written} do not match declared marker plan {keys}"
    )
    for item in plan:
        marker_column = (int(item["x"]) >> 4, int(item["z"]) >> 4)
        require(
            marker_column in occupied[item["key"]], f"{item['key']} marker column is not occupied"
        )
    dimensions = [_dimension_fact(world, item) for item in plan]
    return {"clean_shutdown": True, "written_dimension_keys": written, "dimensions": dimensions}


def verify_official_world(
    world: Path, plan: Sequence[Mapping[str, Any]], bbb: Mapping[str, Any]
) -> dict[str, Any]:
    keys = list(bbb["written_dimension_keys"])
    require(
        [item["key"] for item in plan] == keys,
        "official marker plan differs from the BBB written key set",
    )
    occupied = occupied_dimension_columns(world, keys)
    require(
        [key for key in keys if occupied[key]] == keys,
        "official occupied dimension key set differs from BBB",
    )
    previous = {item["key"]: item for item in bbb["dimensions"]}
    dimensions = [_dimension_fact(world, item, previous=previous[item["key"]]) for item in plan]
    return {
        "clean_startup": True,
        "clean_shutdown": True,
        "loaded_dimension_keys": keys,
        "saved_dimension_keys": keys,
        "dimensions": dimensions,
    }


def wait_log(
    path: Path,
    marker: str,
    process: subprocess.Popen[bytes],
    timeout: float,
    *,
    after: int = 0,
) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = path.read_text(encoding="utf-8", errors="replace") if path.is_file() else ""
        if marker in text[after:]:
            return
        if process.poll() is not None:
            raise DimensionGateError(f"process exited before {marker!r}; log: {path}")
        time.sleep(0.1)
    raise DimensionGateError(f"timed out waiting for {marker!r}; log: {path}")


def send_until_log(
    process: subprocess.Popen[bytes],
    log: Path,
    command: str,
    marker: str,
    timeout: float,
) -> None:
    require(process.stdin is not None, "official server has no command input")
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        text = log.read_text(encoding="utf-8", errors="replace") if log.is_file() else ""
        if marker in text:
            return
        require(process.poll() is None, f"official server exited before {marker!r}")
        process.stdin.write((command + "\n").encode())
        process.stdin.flush()
        time.sleep(0.25)
    raise DimensionGateError(f"timed out waiting for {marker!r}; log: {log}")


def run_probe(
    client_bin: Path, address: str, game_dir: Path, output: Path, timeout: float
) -> dict[str, Any]:
    game_dir.mkdir(parents=True)
    command = [
        str(client_bin),
        "--game-dir",
        str(game_dir),
        "--server",
        address,
        "--username",
        PROBE_USERNAME,
        "--probe-server",
        "--probe-timeout-seconds",
        str(int(timeout)),
        "--probe-server-dimension-transitions",
    ]
    completed = subprocess.run(command, check=False, capture_output=True, timeout=timeout + 10)
    output.write_bytes(completed.stdout)
    if completed.returncode != 0:
        raise DimensionGateError(
            f"dimension probe failed: {completed.stderr.decode('utf-8', 'replace')[:600]}"
        )
    return strict_json(output)


def write_operator(repo_root: Path, directory: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    command = [
        str(repo_root / "scripts/python"),
        "python",
        str(repo_root / "scripts/playable_gate.py"),
        "write-operator",
        "--output",
        str(directory / "ops.json"),
        "--username",
        PROBE_USERNAME,
    ]
    completed = subprocess.run(command, check=False, capture_output=True, timeout=60)
    if completed.returncode != 0:
        raise DimensionGateError(
            f"could not write operator fixture: {completed.stderr.decode('utf-8', 'replace')}"
        )


def command_report(admin_bin: Path, endpoint: str, command: str) -> dict[str, Any]:
    completed = subprocess.run(
        [
            str(admin_bin),
            "--endpoint",
            endpoint,
            "call",
            "command.execute",
            "--arguments-json",
            json.dumps({"command": command}, separators=(",", ":")),
        ],
        check=False,
        capture_output=True,
        timeout=60,
    )
    if completed.returncode != 0:
        raise DimensionGateError(
            f"bbb-admin failed /{command}: {completed.stderr.decode('utf-8', 'replace')[:500]}"
        )
    try:
        answer = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise DimensionGateError(f"bbb-admin returned invalid JSON for /{command}") from error
    report = answer.get("command") if isinstance(answer, Mapping) else None
    require(
        isinstance(report, dict) and type(report.get("result")) is int and report["result"] > 0,
        f"BBB command /{command} did not succeed: {answer!r}",
    )
    return report


def _bbb_log_complaints(text: str) -> list[str]:
    return [
        line
        for line in text.splitlines()
        if re.search(r'"level"\s*:\s*"ERROR"', line)
        or re.search(
            r"\b(?:panicked at|panic\s*:|fatal(?: runtime)? error|"
            r"unhandled exception|exception(?: in thread)?\s*:)",
            line,
            re.IGNORECASE,
        )
    ]


def stop_bbb(
    admin_bin: Path, endpoint: str, process: subprocess.Popen[bytes], log: Path, timeout: float
) -> None:
    completed = subprocess.run(
        [str(admin_bin), "--endpoint", endpoint, "call", "server.stop"],
        check=False,
        capture_output=True,
        timeout=60,
    )
    require(
        completed.returncode == 0,
        f"bbb-admin server.stop failed: {completed.stderr.decode('utf-8', 'replace')[:500]}",
    )
    try:
        code = process.wait(timeout=timeout)
    except subprocess.TimeoutExpired as error:
        process.kill()
        raise DimensionGateError("BBB did not stop through management") from error
    require(code == 0, f"BBB exited {code} after server.stop")
    text = log.read_text(encoding="utf-8", errors="replace")
    require(
        '"event":"world.saved"' in text and '"event":"server.stopped"' in text,
        "BBB clean stop did not log world.saved and server.stopped",
    )
    require('"event":"world.save_failed"' not in text, "BBB reported a save failure")
    require(
        text.index('"event":"world.saved"') < text.index('"event":"server.stopped"'),
        "BBB logged server.stopped before world.saved",
    )
    complaints = _bbb_log_complaints(text)
    require(not complaints, f"BBB server logged errors: {complaints[:4]}")


def _terminate(process: subprocess.Popen[bytes] | None) -> None:
    if process is None or process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=30)
    except subprocess.TimeoutExpired:
        process.kill()


def live(args: argparse.Namespace) -> dict[str, Any]:
    require(
        os.environ.get("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA") == "1",
        "live mode requires BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1",
    )
    repo_root = args.repo_root.resolve()
    artifact = args.artifact_dir.resolve()
    require(
        not artifact.exists() or not any(artifact.iterdir()),
        f"artifact directory is not empty: {artifact}",
    )
    artifact.mkdir(parents=True, exist_ok=True)
    bbb_client_world = artifact / "bbb-client-world"
    bbb_client_world.mkdir()
    bbb_admin = artifact / "bbb-admin"
    write_operator(repo_root, bbb_admin)
    bbb_port, mcp_port = free_port(), free_port()
    while mcp_port == bbb_port:
        mcp_port = free_port()
    bbb_log = artifact / "bbb.log"
    endpoint = f"http://127.0.0.1:{mcp_port}/mcp"
    bbb_process: subprocess.Popen[bytes] | None = None
    official_process: subprocess.Popen[bytes] | None = None
    try:
        handle = bbb_log.open("wb")
        bbb_process = subprocess.Popen(
            [
                str(args.server_bin),
                "--bind",
                f"127.0.0.1:{bbb_port}",
                "--mcp-bind",
                f"127.0.0.1:{mcp_port}",
                "--mcp-loopback-scope",
                "command",
                "--world",
                str(bbb_client_world),
                "--level-type",
                "flat",
                "--max-players",
                "2",
                "--difficulty",
                args.difficulty,
                "--gamemode",
                args.gamemode,
                "--force-gamemode",
                "--admin-dir",
                str(bbb_admin),
                # Forcing a chunk a thousand blocks out is answered from the
                # tick, and the tick before it is still generating the previous
                # dimension's marker; the ten-second default is a caller
                # deadline, not a budget for the work.
                "--mcp-request-timeout-seconds",
                "120",
                "--view-distance",
                "2",
                "--simulation-distance",
                "2",
                "--pause-when-empty-seconds",
                "0",
            ],
            stdin=subprocess.DEVNULL,
            stdout=handle,
            stderr=subprocess.STDOUT,
        )
        handle.close()
        wait_log(bbb_log, '"event":"server.ready"', bbb_process, args.timeout)
        bbb_probe = run_probe(
            args.client_bin,
            f"127.0.0.1:{bbb_port}",
            artifact / "bbb-client",
            artifact / "bbb-probe.json",
            args.timeout,
        )
        plan = marker_plan(bbb_probe)
        for item in plan:
            prefix = f"execute in {item['key']} run"
            command_report(
                args.admin_bin, endpoint, f"{prefix} forceload add {item['x']} {item['z']}"
            )
            command_report(
                args.admin_bin,
                endpoint,
                f"{prefix} setblock {item['x']} {item['y']} {item['z']} minecraft:diamond_block",
            )
        stop_bbb(args.admin_bin, endpoint, bbb_process, bbb_log, args.timeout)
        bbb_process = None
        bbb_facts = inspect_bbb_client_world(bbb_client_world, plan)

        official_dir = artifact / "official"
        official_world = official_dir / "world"
        shutil.copytree(bbb_client_world, official_world)
        (official_world / "session.lock").unlink(missing_ok=True)
        write_operator(repo_root, official_dir)
        properties = Path(args.shared_properties).read_text(encoding="utf-8")
        official_port = free_port()
        (official_dir / "server.properties").write_text(
            properties
            + f"\nserver-port={official_port}\nlevel-name=world\nmotd=bbb dimension runtime 26.1\n",
            encoding="utf-8",
        )
        (official_dir / "eula.txt").write_text("eula=true\n", encoding="utf-8")
        official_log = artifact / "official.log"
        handle = official_log.open("wb")
        official_process = subprocess.Popen(
            [str(args.java_bin), "-Xms512M", "-Xmx1G", "-jar", str(args.server_jar), "nogui"],
            cwd=official_dir,
            stdin=subprocess.PIPE,
            stdout=handle,
            stderr=subprocess.STDOUT,
        )
        handle.close()
        wait_log(official_log, "Done (", official_process, args.timeout)
        require(official_process.stdin is not None, "official server has no command input")
        for index, item in enumerate(plan):
            forceload = f"execute in {item['key']} run forceload add {item['x']} {item['z']}"
            official_process.stdin.write((forceload + "\n").encode())
            official_process.stdin.flush()
            readback = (
                f"execute in {item['key']} if block "
                f"{item['x']} {item['y']} {item['z']} minecraft:diamond_block "
                f"run say bbb-dimension-loaded-{index}"
            )
            send_until_log(
                official_process,
                official_log,
                readback,
                f"bbb-dimension-loaded-{index}",
                args.timeout,
            )
            mutation = (
                f"execute in {item['key']} run setblock "
                f"{item['x'] + 1} {item['y']} {item['z']} minecraft:gold_block"
            )
            official_process.stdin.write((mutation + "\n").encode())
            official_process.stdin.flush()
            confirm_mutation = (
                f"execute in {item['key']} if block "
                f"{item['x']} {item['y']} {item['z']} minecraft:diamond_block "
                f"if block {item['x'] + 1} {item['y']} {item['z']} "
                f"minecraft:gold_block run say official-dimension-mutated-{index}"
            )
            send_until_log(
                official_process,
                official_log,
                confirm_mutation,
                f"official-dimension-mutated-{index}",
                args.timeout,
            )

        official_probe = run_probe(
            args.client_bin,
            f"127.0.0.1:{official_port}",
            artifact / "official-client",
            artifact / "official-probe.json",
            args.timeout,
        )
        compared = compare_probes(bbb_probe, official_probe)
        published_at = max(item["published_at"] for item in bbb_facts["dimensions"])
        deadline = time.monotonic() + args.timeout
        while int(time.time()) <= published_at:
            require(
                time.monotonic() < deadline,
                "wall clock did not cross the BBB region timestamp second",
            )
            time.sleep(0.05)
        before_flush = official_log.read_text(encoding="utf-8", errors="replace")
        # SaveAllCommand.java:23-32; MinecraftServer.java:586-613
        official_process.stdin.write(b"save-all flush\n")
        official_process.stdin.flush()
        wait_log(
            official_log,
            "Saved the game",
            official_process,
            args.timeout,
            after=len(before_flush),
        )
        wait_log(
            official_log,
            "All dimensions are saved",
            official_process,
            args.timeout,
            after=len(before_flush),
        )
        after_flush = official_log.read_text(encoding="utf-8", errors="replace")
        require(
            after_flush.startswith(before_flush), "official log changed before the flush boundary"
        )
        flush_output = after_flush[len(before_flush) :]
        require(
            "Saved the game" in flush_output and "All dimensions are saved" in flush_output,
            "official flush completion was not logged after save-all flush",
        )
        require(
            flush_output.index("All dimensions are saved") < flush_output.index("Saved the game"),
            "official save success was logged before the all-dimensions flush barrier",
        )
        official_process.stdin.write(b"stop\n")
        official_process.stdin.flush()
        # MinecraftServer.java:632-690
        code = official_process.wait(timeout=args.timeout)
        require(code == 0, f"official server exited {code}")
        official_process = None
        log_text = official_log.read_text(encoding="utf-8", errors="replace")
        complaints = [
            line
            for line in log_text.splitlines()
            if re.search(r"ERROR|Exception|Caused by|Failed to", line)
        ]
        require(not complaints, f"official server logged errors: {complaints[:4]}")
        official_facts = verify_official_world(official_world, plan, bbb_facts)
        observation = {
            "artifact_kind": "server-dimension-runtime-official-observation",
            "official_server_jar_sha256": hashlib.sha256(
                Path(args.server_jar).read_bytes()
            ).hexdigest(),
            "configuration": compared["configuration"],
            "transitions": compared["transitions"],
            "bbb": bbb_facts,
            "official": official_facts,
        }
        observation_path = artifact / "observation.json"
        atomic_write(
            observation_path,
            (json.dumps(observation, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode(
                "utf-8"
            ),
        )
        return {"observation": str(observation_path), "artifact_dir": str(artifact)}
    finally:
        _terminate(bbb_process)
        _terminate(official_process)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    subparsers = parser.add_subparsers(dest="action", required=True)
    live_parser = subparsers.add_parser("live")
    live_parser.add_argument("--artifact-dir", type=Path, required=True)
    live_parser.add_argument("--client-bin", type=Path, required=True)
    live_parser.add_argument("--server-bin", type=Path, required=True)
    live_parser.add_argument("--admin-bin", type=Path, required=True)
    live_parser.add_argument("--java-bin", type=Path, required=True)
    live_parser.add_argument("--server-jar", type=Path, required=True)
    live_parser.add_argument("--shared-properties", type=Path, required=True)
    live_parser.add_argument("--timeout", type=float, default=90)
    live_parser.add_argument("--gamemode", default="creative")
    live_parser.add_argument("--force-gamemode", action="store_true")
    live_parser.add_argument("--difficulty", default="peaceful")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        print(json.dumps(live(args), sort_keys=True))
    except (
        DimensionGateError,
        ValueError,
        OSError,
        anvil.AnvilError,
        subprocess.TimeoutExpired,
    ) as error:
        print(f"dimension runtime gate: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
