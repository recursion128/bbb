"""Unit coverage for the round trip's entity step.

Everything a two-server run can prove about this step costs an official world
capture and two server boots. What can be wrong without any of that is the part
this file pins: the scenario the official server is told to place, the flight
arithmetic the step count is chosen from, and -- above all -- the assertions,
because their whole job is to fail on shapes a passing run would otherwise be
indistinguishable from. An entity storage that is missing, empty or full of the
wrong thing all read as "no falling block", and each has to say which.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any

SCRIPTS = Path(__file__).resolve().parents[1]
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

from bbb_tools.anvil import SECTOR_BYTES, chunk_timestamp, slot_index  # noqa: E402
from server_round_trip_entity import (  # noqa: E402
    FALLING_BLOCK_ID,
    FALLING_SAND,
    REQUIRED_FIELDS,
    SAND_BLOCK,
    SAND_X,
    SAND_Y,
    SAND_Z,
    STEP_TICKS,
    RoundTripEntityError,
    numbers,
    probe,
    require_entity_storage,
    require_falling_block,
    require_not_behind,
    require_republished,
    require_staged_scenario,
    setup_lines,
    write_report,
)
from server_world_oracle import DIMENSION, ENTITY_FIELDS  # noqa: E402

#: What the flat capture's surface is. `FlatLevelGeneratorSettings.getDefault`
#: lays bedrock, two dirt and a grass block from the world's floor, so the
#: highest solid block is y=-61 and a falling block comes to rest at y=-60.
GROUND_Y = -60


def record(**overrides: Any) -> dict[str, Any]:
    """A saved falling block of the shape both servers write, with edits."""
    written = {
        "id": FALLING_BLOCK_ID,
        "Pos": [SAND_X + 0.5, 225.0, SAND_Z + 0.5],
        "Motion": [0.0, -1.55, 0.0],
        "OnGround": 0,
        "BlockState": {"Name": SAND_BLOCK},
        "Time": 78,
    }
    written.update(overrides)
    return written


def fall(ticks: int) -> float:
    """How far a falling block drops in `ticks`, by vanilla's own arithmetic.

    `FallingBlockEntity.tick` adds gravity, moves by the result and only then
    scales the speed: `applyGravity` (Entity.java:1440-1445) with the 0.04 of
    `getDefaultGravity` (FallingBlockEntity.java:142-144), `move` with the
    delta (FallingBlockEntity.java:154), and the 0.98 at the end of the tick
    (FallingBlockEntity.java:240).
    """
    speed = 0.0
    dropped = 0.0
    for _ in range(ticks):
        speed -= 0.04
        dropped -= speed
        speed *= 0.98
    return dropped


class ScenarioTests(unittest.TestCase):
    def test_setup_forceloads_the_column_the_sand_falls_down(self) -> None:
        self.assertEqual(
            [
                f"forceload add {(SAND_X >> 4) * 16} {(SAND_Z >> 4) * 16}",
                f"setblock {SAND_X} {SAND_Y} {SAND_Z} {SAND_BLOCK}",
            ],
            setup_lines(),
        )
        self.assertEqual((SAND_X >> 4, SAND_Z >> 4), (probe().chunk_x, probe().chunk_z))

    def test_the_freeze_is_not_in_the_setup_lines(self) -> None:
        # It is sent and waited for first, so that no tick runs between the
        # forceload and the block that schedules the fall.
        self.assertEqual([], [line for line in setup_lines() if "tick" in line])

    def test_the_scenario_reads_the_column_it_writes(self) -> None:
        self.assertIsNotNone(FALLING_SAND.entities)
        self.assertEqual(FALLING_SAND.box.chunk_columns(), [(probe().chunk_x, probe().chunk_z)])

    def test_required_fields_are_the_oracle_s_own(self) -> None:
        self.assertEqual(ENTITY_FIELDS, REQUIRED_FIELDS)
        for field in ("id", "Pos", "Motion", "BlockState", "Time"):
            self.assertIn(field, REQUIRED_FIELDS)

    def test_the_step_leaves_the_block_airborne_with_room_to_spare(self) -> None:
        # The lower bound: the step has to outrun the handful of ticks bbb runs
        # between being ready and its freeze landing, and the two the block
        # spends scheduled (FallingBlock.java:59-61).
        self.assertGreater(STEP_TICKS, 20)
        # The upper bound: what is left of the flight after the step is the
        # budget for the official server's own boot-to-freeze window.
        drop = SAND_Y - GROUND_Y
        self.assertLess(fall(STEP_TICKS), drop)
        landing = next(ticks for ticks in range(STEP_TICKS, 2000) if fall(ticks) >= drop)
        self.assertGreater(landing - STEP_TICKS, 100)
        # And the whole flight stays inside the 600 ticks after which
        # `FallingBlockEntity.tick` discards it outright
        # (FallingBlockEntity.java:172-177).
        self.assertLess(landing, 600)


class RecordTests(unittest.TestCase):
    def assertNamed(self, fragment: str, records: list[dict[str, Any]]) -> None:
        with self.assertRaises(RoundTripEntityError) as raised:
            require_falling_block(records, actor="bbb-server", chunk=(2, 0))
        self.assertIn(fragment, str(raised.exception))
        self.assertIn("bbb-server", str(raised.exception))

    def test_a_written_falling_block_is_returned(self) -> None:
        written = record()
        self.assertEqual(
            written, require_falling_block([written], actor="bbb-server", chunk=(2, 0))
        )

    def test_a_chunk_with_no_records_is_named(self) -> None:
        self.assertNamed("holds no entity records at all", [])

    def test_records_without_a_falling_block_are_named(self) -> None:
        self.assertNamed(
            f"none of them is {FALLING_BLOCK_ID}",
            [{"id": "minecraft:item"}, {"id": "minecraft:cow"}],
        )

    def test_a_second_falling_block_is_named(self) -> None:
        self.assertNamed("records; the scenario places", [record(), record()])

    def test_every_missing_field_is_named(self) -> None:
        for field in REQUIRED_FIELDS:
            written = record()
            del written[field]
            with self.subTest(field=field):
                if field == "id":
                    # Without an id it is not a falling block at all, which is
                    # the earlier and equally loud failure.
                    self.assertNamed(f"none of them is {FALLING_BLOCK_ID}", [written])
                else:
                    self.assertNamed(f"is missing {field}", [written])

    def test_a_position_that_is_not_three_doubles_is_named(self) -> None:
        self.assertNamed("Pos that is not three doubles", [record(Pos=[1.0, 2.0])])
        self.assertNamed("Pos that is not three doubles", [record(Pos="34,300,8")])

    def test_a_motion_that_is_not_three_doubles_is_named(self) -> None:
        self.assertNamed("Motion that is not three doubles", [record(Motion=[0.0, -1.0])])

    def test_a_record_that_never_moved_is_named(self) -> None:
        self.assertNamed("is not falling", [record(Motion=[0.0, 0.0, 0.0])])

    def test_a_foreign_block_state_is_named(self) -> None:
        self.assertNamed("rather than the", [record(BlockState={"Name": "minecraft:gravel"})])
        self.assertNamed("rather than the", [record(BlockState="minecraft:sand")])

    def test_a_record_that_was_never_ticked_is_named(self) -> None:
        self.assertNamed("has Time 0", [record(Time=0)])
        self.assertNamed("has Time True", [record(Time=True)])

    def test_numbers_rejects_booleans_and_short_lists(self) -> None:
        self.assertIsNone(numbers([True, False, True], 3))
        self.assertIsNone(numbers([1.0, 2.0], 3))
        self.assertEqual([1.0, 2.0, 3.0], numbers([1, 2.0, 3], 3))


class ReloadTests(unittest.TestCase):
    """What the reload is held against: the save and the record bbb published."""

    def test_a_ticked_reload_reports_what_it_ran(self) -> None:
        written = record(Time=79, Pos=[34.5, 220.135, 8.5])
        reloaded = record(Time=85, Pos=[34.5, 210.403, 8.5])
        advance = require_not_behind(reloaded, written)
        self.assertEqual(6, advance["ticks"])
        self.assertAlmostEqual(9.732, advance["fell"], places=3)

    def test_a_reload_frozen_before_its_first_tick_is_accepted(self) -> None:
        # Whether the entity storage's own load lands before the freeze is a
        # race, so an unchanged record is not a failure here; what says the
        # official server read the save is the write stamp, not the tick.
        written = record(Time=79)
        self.assertEqual({"ticks": 0, "fell": 0.0}, require_not_behind(record(Time=79), written))

    def test_a_record_that_went_backwards_is_named(self) -> None:
        with self.assertRaises(RoundTripEntityError) as raised:
            require_not_behind(record(Time=70), record(Time=79))
        self.assertIn("behind the 79", str(raised.exception))

    def test_a_record_that_rose_is_named(self) -> None:
        written = record(Time=79, Pos=[34.5, 220.0, 8.5])
        with self.assertRaises(RoundTripEntityError) as raised:
            require_not_behind(record(Time=85, Pos=[34.5, 221.0, 8.5]), written)
        self.assertIn("rose", str(raised.exception))

    def test_an_uncomparable_record_is_named(self) -> None:
        with self.assertRaises(RoundTripEntityError) as raised:
            require_not_behind(record(), record(Time="79"))
        self.assertIn("cannot compare", str(raised.exception))
        with self.assertRaises(RoundTripEntityError) as raised:
            require_not_behind(record(Pos=[1.0, 2.0]), record(Time=1))
        self.assertIn("cannot compare", str(raised.exception))

    def test_a_write_report_is_read_back_and_a_broken_one_is_named(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "entity-write.json"
            report.write_text(
                json.dumps({"entity": record(), "published_world": "/nowhere"}), encoding="utf-8"
            )
            self.assertEqual(record(), write_report(report)["entity"])
            report.write_text(json.dumps({"published_world": "/nowhere"}), encoding="utf-8")
            with self.assertRaises(RoundTripEntityError) as raised:
                write_report(report)
            self.assertIn("carries no entity", str(raised.exception))
            report.write_text(json.dumps({"entity": record()}), encoding="utf-8")
            with self.assertRaises(RoundTripEntityError) as raised:
                write_report(report)
            self.assertIn("does not say which save", str(raised.exception))
            with self.assertRaises(RoundTripEntityError) as raised:
                write_report(Path(directory) / "absent.json")
            self.assertIn("cannot read the write report", str(raised.exception))

    def entities_region(self, root: Path, stamp: int | None) -> Path:
        """A save whose entity region file stamps the scenario column, or not."""
        directory = root / "dimensions" / "minecraft" / DIMENSION / "entities"
        directory.mkdir(parents=True)
        header = bytearray(SECTOR_BYTES * 2)
        if stamp is not None:
            index = slot_index(probe().chunk_x, probe().chunk_z) * 4
            # An unwritten slot has no sector either, and `chunk_timestamp`
            # reads the stamp alone, so only the second sector matters here.
            header[SECTOR_BYTES + index : SECTOR_BYTES + index + 4] = stamp.to_bytes(4, "big")
        (directory / "r.0.0.mca").write_bytes(bytes(header))
        return root

    def test_the_write_stamp_is_read_out_of_the_second_header_sector(self) -> None:
        # `RegionFile` re-reads the header buffer as its timestamps from
        # position 4096 (RegionFile.java:61-64), so the stamp for a slot lives
        # one sector past that slot's offset entry.
        with tempfile.TemporaryDirectory() as directory:
            root = self.entities_region(Path(directory) / "save", 1787082551)
            entities = root / "dimensions" / "minecraft" / DIMENSION / "entities"
            self.assertEqual(
                1787082551, chunk_timestamp(entities, probe().chunk_x, probe().chunk_z)
            )
            # A column nothing ever wrote reads as no stamp rather than as zero.
            self.assertIsNone(chunk_timestamp(entities, 31, 31))
            self.assertIsNone(chunk_timestamp(entities / "absent", 0, 0))

    def test_a_republished_column_reports_both_stamps(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            published = self.entities_region(Path(directory) / "published", 1787082551)
            reloaded = self.entities_region(Path(directory) / "reloaded", 1787082566)
            self.assertEqual(
                {"published_at": 1787082551, "republished_at": 1787082566},
                require_republished(published, reloaded),
            )

    def test_a_column_the_official_server_never_wrote_is_named(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            published = self.entities_region(Path(directory) / "published", 1787082551)
            reloaded = self.entities_region(Path(directory) / "reloaded", 1787082551)
            with self.assertRaises(RoundTripEntityError) as raised:
                require_republished(published, reloaded)
        message = str(raised.exception)
        self.assertIn("still carries the stamp bbb-server wrote (1787082551)", message)
        self.assertIn("never republished", message)

    def test_a_column_with_no_stamp_at_all_is_named(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            published = self.entities_region(Path(directory) / "published", None)
            reloaded = self.entities_region(Path(directory) / "reloaded", 1787082566)
            with self.assertRaises(RoundTripEntityError) as raised:
                require_republished(published, reloaded)
        self.assertIn("holds no write stamp", str(raised.exception))


class StorageTests(unittest.TestCase):
    def entities_dir(self, world: Path) -> Path:
        return world / "dimensions" / "minecraft" / DIMENSION / "entities"

    def test_a_save_with_no_entity_storage_is_named(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(RoundTripEntityError) as raised:
                require_entity_storage(Path(directory), actor="bbb-server")
        message = str(raised.exception)
        self.assertIn("wrote no entity storage at all", message)
        self.assertIn("bbb-server", message)

    def test_an_entity_directory_with_no_chunk_is_named(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            world = Path(directory)
            self.entities_dir(world).mkdir(parents=True)
            with self.assertRaises(RoundTripEntityError) as raised:
                require_entity_storage(world, actor="the official 26.1 server")
        message = str(raised.exception)
        self.assertIn(f"wrote no entity chunk at ({probe().chunk_x}, {probe().chunk_z})", message)
        self.assertIn("the official 26.1 server", message)

    def test_a_staged_save_without_the_sand_is_named(self) -> None:
        # A `/setblock` the official server refused would leave a world that
        # ticks into an empty entity storage, which is the failure that looks
        # exactly like a runtime that does not work.
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(RoundTripEntityError) as raised:
                require_staged_scenario(Path(directory))
        message = str(raised.exception)
        self.assertIn("<unwritten>", message)
        self.assertIn(f"({SAND_X}, {SAND_Y}, {SAND_Z})", message)
        self.assertIn(SAND_BLOCK, message)

    def test_an_empty_region_file_is_the_same_named_failure(self) -> None:
        # `RegionFile` creates the file when the dimension opens the directory
        # and only writes its header once a chunk lands in it, so this is what a
        # server that ticked no entity into the column leaves behind.
        with tempfile.TemporaryDirectory() as directory:
            world = Path(directory)
            entities = self.entities_dir(world)
            entities.mkdir(parents=True)
            (entities / "r.0.0.mca").write_bytes(b"")
            with self.assertRaises(RoundTripEntityError) as raised:
                require_entity_storage(world, actor="bbb-server")
        self.assertIn("wrote no entity chunk", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
