"""Focused tests for the Noise FULL official-resume helper."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import server_round_trip_noise as NOISE  # noqa: E402

MARKER = {"x": 35, "y": 81, "z": -17}


def probe_report(**overrides: object) -> dict[str, object]:
    replay: dict[str, object] = {
        "placed_block": MARKER,
        "placed_block_name": "minecraft:stone",
        "note_block_name": "minecraft:note_block",
    }
    replay.update(overrides)
    return {"interaction_replay": replay}


def full_chunk(**overrides: object) -> dict[str, object]:
    chunk_x, chunk_z = MARKER["x"] >> 4, MARKER["z"] >> 4
    chunk: dict[str, object] = {
        "DataVersion": 4786,
        "xPos": chunk_x,
        "zPos": chunk_z,
        "yPos": -4,
        "Status": "minecraft:full",
        "isLightOn": 1,
        "sections": [
            {"Y": section_y, "block_states": {}, "biomes": {}}
            for section_y in range(-4, 20)
        ],
        "block_entities": [],
        "block_ticks": [],
        "fluid_ticks": [],
        "PostProcessing": [[] for _ in range(24)],
        "Heightmaps": {name: [0] * 37 for name in NOISE.FINAL_HEIGHTMAPS},
        "structures": {"starts": {}, "References": {}},
    }
    chunk.update(overrides)
    return chunk


class FakeReader:
    states: dict[tuple[int, int, int], str] = {}

    def __init__(self, world: Path, dimension: str) -> None:
        self.world_dir = world
        self.dimension = dimension
        self.region_dir = world / "dimensions" / "minecraft" / dimension / "region"

    def block_state(self, x: int, y: int, z: int) -> SimpleNamespace | None:
        name = self.states.get((x, y, z))
        return None if name is None else SimpleNamespace(name=name)


class PrepareTests(unittest.TestCase):
    def make_world(self, root: Path) -> Path:
        world = root / "input"
        dimension = world / "dimensions" / "minecraft" / "overworld"
        for name in NOISE.STORAGE_NAMES:
            storage = dimension / name
            storage.mkdir(parents=True)
            (storage / "r.0.0.mca").write_bytes(name.encode())
            (storage / "c.0.0.mcc").write_bytes(b"external")
        (world / "level.dat").write_bytes(b"normal generator metadata")
        (world / "session.lock").write_bytes(b"locked")
        (world / "datapacks" / "pack").mkdir(parents=True)
        (world / "datapacks" / "pack" / "pack.mcmeta").write_text("{}", encoding="utf-8")
        return world

    def test_prepare_copies_metadata_and_clears_only_the_output_storage(self) -> None:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        source = self.make_world(root)
        before = {
            path.relative_to(source).as_posix(): path.read_bytes()
            for path in source.rglob("*")
            if path.is_file()
        }
        output = root / "prepared"

        report = NOISE.prepare(source, output)

        after = {
            path.relative_to(source).as_posix(): path.read_bytes()
            for path in source.rglob("*")
            if path.is_file()
        }
        self.assertEqual(before, after)
        self.assertFalse((output / "session.lock").exists())
        self.assertEqual(b"normal generator metadata", (output / "level.dat").read_bytes())
        self.assertTrue((output / "datapacks" / "pack" / "pack.mcmeta").is_file())
        self.assertEqual([], list(output.rglob("*.mca")))
        self.assertEqual([], list(output.rglob("*.mcc")))
        for name in NOISE.STORAGE_NAMES:
            self.assertTrue(
                (output / "dimensions" / "minecraft" / "overworld" / name).is_dir()
            )
        self.assertEqual(str(source.resolve()), report["input_world"])
        self.assertEqual(str(output.resolve()), report["output_world"])
        self.assertEqual(NOISE._tree_digest(source), report["input_sha256"])

    def test_prepare_fails_closed_on_an_existing_or_nested_output(self) -> None:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        source = self.make_world(root)
        existing = root / "existing"
        existing.mkdir()
        sentinel = existing / "keep"
        sentinel.write_text("mine", encoding="utf-8")
        with self.assertRaisesRegex(NOISE.NoiseResumeError, "already exists"):
            NOISE.prepare(source, existing)
        self.assertEqual("mine", sentinel.read_text(encoding="utf-8"))

        with self.assertRaisesRegex(NOISE.NoiseResumeError, "disjoint"):
            NOISE.prepare(source, source / "copied")
        self.assertFalse((source / "copied").exists())

    def test_prepare_rejects_chunk_storage_outside_the_owned_overworld_paths(self) -> None:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        source = self.make_world(root)
        foreign = source / "dimensions" / "minecraft" / "the_nether" / "region"
        foreign.mkdir(parents=True)
        (foreign / "r.0.0.mca").write_bytes(b"foreign")
        with self.assertRaisesRegex(NOISE.NoiseResumeError, "outside the cleared"):
            NOISE.prepare(source, root / "prepared")


class ProbeTests(unittest.TestCase):
    def write_probe(self, root: Path, report: object) -> Path:
        path = root / "probe.json"
        path.write_text(json.dumps(report), encoding="utf-8")
        return path

    def test_marker_is_read_from_the_raw_probe_report(self) -> None:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.assertEqual(MARKER, NOISE.marker_from_probe(self.write_probe(root, probe_report())))

    def test_missing_or_unproven_markers_are_rejected(self) -> None:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        for name, report, fragment in (
            ("replay", {}, "interaction_replay"),
            ("coordinates", probe_report(placed_block=None), "coordinates"),
            ("stone", probe_report(placed_block_name="minecraft:dirt"), "placed_block_name"),
            ("note", probe_report(note_block_name=None), "note_block_name"),
        ):
            with self.subTest(name=name):
                path = root / f"{name}.json"
                path.write_text(json.dumps(report), encoding="utf-8")
                with self.assertRaisesRegex(NOISE.NoiseResumeError, fragment):
                    NOISE.marker_from_probe(path)

    def test_boolean_coordinate_is_not_an_integer(self) -> None:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        marker = dict(MARKER)
        marker["x"] = True
        with self.assertRaisesRegex(NOISE.NoiseResumeError, "must be an integer"):
            NOISE.marker_from_probe(self.write_probe(root, probe_report(placed_block=marker)))


class SchemaTests(unittest.TestCase):
    def test_full_levelchunk_schema_is_summarized(self) -> None:
        summary = NOISE.require_full_schema(full_chunk(), MARKER["x"] >> 4, MARKER["z"] >> 4)
        self.assertEqual(4786, summary["data_version"])
        self.assertEqual("minecraft:full", summary["status"])
        self.assertEqual(24, summary["sections"])
        self.assertEqual(0, summary["light_arrays"])
        self.assertEqual(
            {name: 37 for name in sorted(NOISE.FINAL_HEIGHTMAPS)}, summary["heightmaps"]
        )

    def assertBad(self, chunk: dict[str, object], fragment: str) -> None:
        with self.assertRaisesRegex(NOISE.NoiseResumeError, fragment):
            NOISE.require_full_schema(chunk, MARKER["x"] >> 4, MARKER["z"] >> 4)

    def test_bad_scalar_schema_is_rejected(self) -> None:
        for tag, value in (
            ("DataVersion", 4785),
            ("xPos", 77),
            ("zPos", 77),
            ("yPos", 0),
            ("Status", "minecraft:noise"),
            ("isLightOn", 0),
        ):
            with self.subTest(tag=tag):
                self.assertBad(full_chunk(**{tag: value}), tag)

    def test_missing_or_wrong_shaped_required_tags_are_rejected(self) -> None:
        for tag, kind in NOISE.REQUIRED_TAGS.items():
            with self.subTest(tag=tag, case="missing"):
                chunk = full_chunk()
                del chunk[tag]
                self.assertBad(chunk, tag)
            with self.subTest(tag=tag, case="type"):
                wrong: object = [] if kind is dict else {}
                self.assertBad(full_chunk(**{tag: wrong}), tag)

    def test_proto_only_tags_are_rejected(self) -> None:
        for tag in NOISE.PROTO_ONLY_TAGS:
            with self.subTest(tag=tag):
                self.assertBad(full_chunk(**{tag: []}), "ProtoChunk-only")

    def test_heightmap_set_and_lengths_are_exact(self) -> None:
        maps = deepcopy(full_chunk()["Heightmaps"])
        assert isinstance(maps, dict)
        maps["WORLD_SURFACE_WG"] = [0] * 37
        self.assertBad(full_chunk(Heightmaps=maps), "Heightmaps keys")
        maps = deepcopy(full_chunk()["Heightmaps"])
        assert isinstance(maps, dict)
        maps["OCEAN_FLOOR"] = [0] * 36
        self.assertBad(full_chunk(Heightmaps=maps), "exactly 37 longs")

    def test_section_y_values_are_complete_unique_and_bounded(self) -> None:
        sections = deepcopy(full_chunk()["sections"])
        assert isinstance(sections, list)
        sections.append(deepcopy(sections[0]))
        self.assertBad(full_chunk(sections=sections), "duplicate section Y")

        sections = deepcopy(full_chunk()["sections"])
        assert isinstance(sections, list) and isinstance(sections[0], dict)
        sections[0]["Y"] = -6
        self.assertBad(full_chunk(sections=sections), "outside -5..20")

        sections = deepcopy(full_chunk()["sections"])
        assert isinstance(sections, list)
        sections.pop()
        self.assertBad(full_chunk(sections=sections), "missing terrain section")

    def test_terrain_and_light_padding_section_shapes_are_exact(self) -> None:
        sections = deepcopy(full_chunk()["sections"])
        assert isinstance(sections, list) and isinstance(sections[0], dict)
        del sections[0]["biomes"]
        self.assertBad(full_chunk(sections=sections), "has no biomes compound")

        sections = deepcopy(full_chunk()["sections"])
        assert isinstance(sections, list)
        sections.append({"Y": -5, "SkyLight": [0] * NOISE.LIGHT_BYTES})
        summary = NOISE.require_full_schema(
            full_chunk(sections=sections), MARKER["x"] >> 4, MARKER["z"] >> 4
        )
        self.assertEqual(1, summary["light_arrays"])
        sections[-1]["BlockLight"] = [0] * (NOISE.LIGHT_BYTES - 1)
        self.assertBad(full_chunk(sections=sections), "not exactly 2048 bytes")

        sections = deepcopy(full_chunk()["sections"])
        assert isinstance(sections, list)
        sections.append({"Y": 20})
        self.assertBad(full_chunk(sections=sections), "carries no light array")

    def test_structures_requires_both_child_compounds(self) -> None:
        for name in ("starts", "References"):
            structures = deepcopy(full_chunk()["structures"])
            assert isinstance(structures, dict)
            del structures[name]
            with self.subTest(name=name):
                self.assertBad(full_chunk(structures=structures), f"no {name} compound")


class InspectVerifyTests(unittest.TestCase):
    def setUp(self) -> None:
        FakeReader.states = {
            (MARKER["x"], MARKER["y"], MARKER["z"]): "minecraft:stone",
            (MARKER["x"], MARKER["y"] + 1, MARKER["z"]): "minecraft:note_block",
            (MARKER["x"], MARKER["y"] + 2, MARKER["z"]): "minecraft:gold_block",
        }

    def fixture(self) -> tuple[Path, Path, Path]:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        world = root / "world"
        probe = root / "probe.json"
        probe.write_text(json.dumps(probe_report()), encoding="utf-8")
        published = root / "published.json"
        return world, probe, published

    def patches(self, chunk: dict[str, object], stamp: int):
        return (
            patch.object(NOISE, "WorldSaveReader", FakeReader),
            patch.object(NOISE, "read_chunk", return_value=chunk),
            patch.object(NOISE, "chunk_timestamp", return_value=stamp),
        )

    def test_inspect_requires_markers_schema_and_a_timestamp(self) -> None:
        world, probe, _ = self.fixture()
        reader_patch, chunk_patch, stamp_patch = self.patches(full_chunk(), 100)
        with reader_patch, chunk_patch, stamp_patch:
            report = NOISE.inspect(world, probe, actor="bbb-server")
        self.assertEqual(MARKER, report["marker"])
        self.assertEqual({"x": 2, "z": -2}, report["chunk"])
        self.assertEqual(100, report["published_at"])
        self.assertEqual(str(world.resolve()), report["published_world"])
        self.assertEqual("minecraft:full", report["schema"]["status"])

    def test_inspect_rejects_a_missing_saved_marker(self) -> None:
        world, probe, _ = self.fixture()
        del FakeReader.states[(MARKER["x"], MARKER["y"], MARKER["z"])]
        reader_patch, chunk_patch, stamp_patch = self.patches(full_chunk(), 100)
        with reader_patch, chunk_patch, stamp_patch:
            with self.assertRaisesRegex(NOISE.NoiseResumeError, "stone"):
                NOISE.inspect(world, probe, actor="bbb-server")

    def test_verify_requires_the_control_write_and_an_advanced_timestamp(self) -> None:
        world, probe, published = self.fixture()
        reader_patch, chunk_patch, stamp_patch = self.patches(full_chunk(), 100)
        with reader_patch, chunk_patch, stamp_patch:
            published.write_text(
                json.dumps(NOISE.inspect(world, probe, actor="bbb-server")), encoding="utf-8"
            )

        reader_patch, chunk_patch, stamp_patch = self.patches(full_chunk(), 102)
        with reader_patch, chunk_patch, stamp_patch:
            report = NOISE.verify(world, published, actor="official 26.1")
        self.assertEqual(100, report["published_at"])
        self.assertEqual(102, report["republished_at"])

        reader_patch, chunk_patch, stamp_patch = self.patches(full_chunk(), 100)
        with reader_patch, chunk_patch, stamp_patch:
            with self.assertRaisesRegex(NOISE.NoiseResumeError, "did not advance"):
                NOISE.verify(world, published, actor="official 26.1")

    def test_verify_rejects_a_missing_gold_control_write(self) -> None:
        world, probe, published = self.fixture()
        reader_patch, chunk_patch, stamp_patch = self.patches(full_chunk(), 100)
        with reader_patch, chunk_patch, stamp_patch:
            published.write_text(
                json.dumps(NOISE.inspect(world, probe, actor="bbb-server")), encoding="utf-8"
            )
        del FakeReader.states[(MARKER["x"], MARKER["y"] + 2, MARKER["z"])]
        reader_patch, chunk_patch, stamp_patch = self.patches(full_chunk(), 101)
        with reader_patch, chunk_patch, stamp_patch:
            with self.assertRaisesRegex(NOISE.NoiseResumeError, "gold_block"):
                NOISE.verify(world, published, actor="official 26.1")


if __name__ == "__main__":
    unittest.main()
