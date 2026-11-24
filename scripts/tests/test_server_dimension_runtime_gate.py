from __future__ import annotations

import importlib.util
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/server-dimension-runtime-gate-26.1.sh"
SPEC = importlib.util.spec_from_file_location(
    "server_dimension_runtime_gate", ROOT / "scripts/server_dimension_runtime_gate.py"
)
assert SPEC and SPEC.loader
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


def level(dimension: str, *, min_y: int = 0, height: int = 16) -> dict:
    return {
        "dimension": dimension,
        "dimension_type_id": 0,
        "dimension_type_name": dimension,
        "dimension_type_logical_height": height,
        "dimension_type_infiniburn_tag": "minecraft:infiniburn_overworld",
        "sea_level": 0,
        "min_y": min_y,
        "height": height,
        "is_debug": False,
        "is_flat": True,
    }


def transitions() -> dict:
    legs = []
    for target in GATE.EXPECTED_SEQUENCE[1:]:
        legs.append(
            {
                "target_dimension": target,
                "respawn_count_delta": 1,
                "data_to_keep": 3,
                "level": level(target),
                "milestone_order": list(GATE.MILESTONES),
                "player_position": [0.5, 80.0, 0.5],
                "target_chunk": {"x": 0, "z": 0},
                "first_chunk": {"x": 0, "z": 0},
                "target_chunk_completed": True,
            }
        )
    return {"initial": level("minecraft:overworld"), "legs": legs}


def configuration() -> dict:
    registries = [
        {
            "registry": f"example:registry_{index:02d}",
            "entries": [
                {"resource_id": "example:a", "canonical_nbt_sha256": "a" * 64},
                {"resource_id": "other:z", "canonical_nbt_sha256": "b" * 64},
            ],
        }
        for index in range(28)
    ]
    return {
        "selected_known_packs": 0,
        "packet_order": [
            "select_known_packs",
            *[f"registry_data:{item['registry']}" for item in registries],
            "update_tags",
            "finish",
        ],
        "registries": registries,
        "tags": [
            {
                "registry": f"example:tag_registry_{index:02d}",
                "tags": [
                    {
                        "tag": f"example:tag_{index:02d}",
                        "entries": ["minecraft:stone"],
                    }
                ],
            }
            for index in range(15)
        ],
    }


def probe() -> dict:
    return {"configuration_catalog": configuration(), "dimension_transitions": transitions()}


def region(path: Path, *, slot: int | None = 0, timestamp: int = 10) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if slot is None:
        path.write_bytes(b"")
        return
    data = bytearray(GATE.anvil.SECTOR_BYTES * 3)
    data[slot * 4 : slot * 4 + 4] = bytes((0, 0, 2, 1))
    offset = 4096 + slot * 4
    data[offset : offset + 4] = timestamp.to_bytes(4, "big")
    path.write_bytes(data)


class ProbeContractTests(unittest.TestCase):
    def test_complete_catalog_and_transition_replay_compare_exactly(self) -> None:
        compared = GATE.compare_probes(probe(), probe())
        self.assertEqual(compared["configuration"]["registry_count"], 28)
        self.assertEqual(compared["transitions"]["sequence"], list(GATE.EXPECTED_SEQUENCE))

    def test_registry_or_tag_difference_fails(self) -> None:
        for field in ("registry", "tag"):
            official = probe()
            if field == "registry":
                official["configuration_catalog"]["registries"][0]["entries"][0][
                    "canonical_nbt_sha256"
                ] = "c" * 64
            else:
                official["configuration_catalog"]["tags"][0]["tags"][0]["entries"] = []
            with (
                self.subTest(field=field),
                self.assertRaisesRegex(GATE.DimensionGateError, "configuration catalogs differ"),
            ):
                GATE.compare_probes(probe(), official)

    def test_configuration_rejects_sha1_and_nonzero_known_packs(self) -> None:
        for field in ("digest", "known_packs", "boolean_known_packs"):
            value = probe()
            if field == "digest":
                value["configuration_catalog"]["registries"][0]["entries"][0][
                    "canonical_nbt_sha256"
                ] = "a" * 40
            elif field == "boolean_known_packs":
                value["configuration_catalog"]["selected_known_packs"] = False
            else:
                value["configuration_catalog"]["selected_known_packs"] = 1
            with self.subTest(field=field), self.assertRaises(GATE.DimensionGateError):
                GATE.compare_probes(value, probe())

    def test_transition_order_keep_mask_and_completion_are_required(self) -> None:
        for field, value, message in (
            ("data_to_keep", 0, "KEEP_ALL_DATA"),
            ("milestone_order", list(reversed(GATE.MILESTONES)), "milestones"),
            ("target_chunk_completed", False, "did not complete"),
        ):
            value_probe = probe()
            value_probe["dimension_transitions"]["legs"][0][field] = value
            with (
                self.subTest(field=field),
                self.assertRaisesRegex(GATE.DimensionGateError, message),
            ):
                GATE.compare_probes(value_probe, probe())

    def test_transition_position_must_be_canonical_and_inside_target_chunk(self) -> None:
        moved = probe()
        moved["dimension_transitions"]["legs"][0]["player_position"] = [16.0, 80.0, 0.0]
        with self.assertRaisesRegex(GATE.DimensionGateError, "player_position"):
            GATE.compare_probes(moved, probe())

        wrong_chunk = probe()
        wrong_chunk["dimension_transitions"]["legs"][0]["target_chunk"] = {"x": 1, "z": 0}
        with self.assertRaisesRegex(GATE.DimensionGateError, "cache-center target chunk"):
            GATE.compare_probes(wrong_chunk, probe())

    def test_marker_plan_carries_each_declared_dimension_geometry(self) -> None:
        plan = GATE.marker_plan(probe())
        self.assertEqual(
            [item["key"] for item in plan], list(dict.fromkeys(GATE.EXPECTED_SEQUENCE))
        )
        self.assertEqual(len({(item["x"], item["z"]) for item in plan}), 3)


class AnvilInspectionTests(unittest.TestCase):
    def test_empty_region_file_is_not_a_written_dimension(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "r.0.0.mca"
            region(path, slot=None)
            self.assertEqual(GATE.occupied_slots(path), [])

    def test_empty_dimension_directory_is_not_written(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            world = Path(raw) / "world"
            (GATE.dimension_root(world, "minecraft:overworld") / "region").mkdir(parents=True)
            self.assertEqual(
                GATE.occupied_dimension_columns(world, ["minecraft:overworld"]),
                {"minecraft:overworld": []},
            )

    def test_occupied_slot_is_resolved_to_its_chunk_coordinate(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "r.-1.2.mca"
            region(path, slot=35)
            with mock.patch.object(Path, "read_bytes", side_effect=AssertionError("unbounded")):
                self.assertEqual(GATE.occupied_slots(path), [(-29, 65)])

    def test_absolute_traversal_and_malformed_dimension_keys_fail(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            for key in (
                "/absolute",
                "..:foo",
                "minecraft:../outside",
                "minecraft:a/../../b",
                "not-a-key",
            ):
                with self.subTest(key=key), self.assertRaises(GATE.DimensionGateError):
                    GATE.dimension_root(Path(raw), key)

    def test_symlink_and_unknown_occupied_root_fail(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            world = Path(raw) / "world"
            known = GATE.dimension_root(world, "minecraft:overworld") / "region"
            known.mkdir(parents=True)
            target = Path(raw) / "outside"
            target.mkdir()
            (known / "linked").symlink_to(target, target_is_directory=True)
            with self.assertRaisesRegex(GATE.DimensionGateError, "symlink"):
                GATE.occupied_dimension_columns(world, ["minecraft:overworld"])
            (known / "linked").unlink()
            unknown = world / "dimensions/example/moon/nested/region/r.0.0.mca"
            region(unknown)
            with self.assertRaisesRegex(GATE.DimensionGateError, "undeclared"):
                GATE.occupied_dimension_columns(world, ["minecraft:overworld"])

    def _world(self, root: Path) -> tuple[Path, list[dict]]:
        world = root / "world"
        plan = [{"key": "minecraft:overworld", "min_y": 0, "height": 16, "x": 8, "y": 8, "z": 8}]
        dimension = GATE.dimension_root(world, plan[0]["key"])
        region(dimension / "region/r.0.0.mca")
        return world, plan

    @mock.patch.object(GATE.anvil, "chunk_timestamp", return_value=10)
    @mock.patch.object(
        GATE.anvil,
        "chunk_block_state",
        return_value=GATE.anvil.BlockState("minecraft:diamond_block"),
    )
    @mock.patch.object(
        GATE.anvil,
        "read_chunk",
        return_value={
            "yPos": 0,
            "sections": [{"Y": 0, "block_states": {}, "biomes": {}}],
        },
    )
    def test_bbb_marker_geometry_and_written_set_close(self, _read, _block, _timestamp) -> None:
        with tempfile.TemporaryDirectory() as raw:
            world, plan = self._world(Path(raw))
            facts = GATE.inspect_bbb_client_world(world, plan)
            self.assertEqual(facts["written_dimension_keys"], ["minecraft:overworld"])
            self.assertEqual(facts["dimensions"][0]["section_y"], [0])

    @mock.patch.object(GATE.anvil, "chunk_timestamp", return_value=10)
    @mock.patch.object(
        GATE.anvil,
        "chunk_block_state",
        return_value=GATE.anvil.BlockState("minecraft:diamond_block"),
    )
    @mock.patch.object(
        GATE.anvil,
        "read_chunk",
        return_value={
            "yPos": -1,
            "sections": [{"Y": 0, "block_states": {}, "biomes": {}}],
        },
    )
    def test_wrong_y_pos_fails(self, _read, _block, _timestamp) -> None:
        with tempfile.TemporaryDirectory() as raw:
            world, plan = self._world(Path(raw))
            with self.assertRaisesRegex(GATE.DimensionGateError, "yPos"):
                GATE.inspect_bbb_client_world(world, plan)

    def test_missing_section_geometry_fails(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            world, plan = self._world(Path(raw))
            plan[0]["height"] = 32
            chunk = {
                "yPos": 0,
                "sections": [{"Y": 0, "block_states": {}, "biomes": {}}],
            }
            with (
                mock.patch.object(GATE.anvil, "read_chunk", return_value=chunk),
                self.assertRaisesRegex(GATE.DimensionGateError, "missing terrain sections"),
            ):
                GATE.inspect_bbb_client_world(world, plan)

    def test_missing_declared_dimension_fails_written_key_set(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            world, plan = self._world(Path(raw))
            plan.append(
                {
                    "key": "minecraft:the_nether",
                    "min_y": 0,
                    "height": 16,
                    "x": 520,
                    "y": 8,
                    "z": 520,
                }
            )
            with self.assertRaisesRegex(GATE.DimensionGateError, "written dimensions"):
                GATE.inspect_bbb_client_world(world, plan)

    def test_missing_marker_and_nonadvancing_timestamp_fail(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            world, plan = self._world(Path(raw))
            chunk = {
                "yPos": 0,
                "sections": [{"Y": 0, "block_states": {}, "biomes": {}}],
            }
            with (
                mock.patch.object(GATE.anvil, "read_chunk", return_value=chunk),
                mock.patch.object(GATE.anvil, "chunk_timestamp", return_value=10),
                mock.patch.object(GATE.anvil, "chunk_block_state", return_value=None),
            ):
                with self.assertRaisesRegex(GATE.DimensionGateError, "diamond"):
                    GATE.inspect_bbb_client_world(world, plan)
            previous = {
                "written_dimension_keys": ["minecraft:overworld"],
                "dimensions": [
                    {
                        "key": "minecraft:overworld",
                        "path": "dimensions/minecraft/overworld",
                        "marker_chunk": {"x": 0, "z": 0},
                        "min_y": 0,
                        "height": 16,
                        "y_pos": 0,
                        "section_y": [0],
                        "published_at": 10,
                    }
                ],
            }
            with (
                mock.patch.object(GATE.anvil, "read_chunk", return_value=chunk),
                mock.patch.object(GATE.anvil, "chunk_timestamp", return_value=10),
                mock.patch.object(
                    GATE.anvil,
                    "chunk_block_state",
                    side_effect=[
                        GATE.anvil.BlockState("minecraft:diamond_block"),
                        GATE.anvil.BlockState("minecraft:gold_block"),
                    ],
                ),
            ):
                with self.assertRaisesRegex(GATE.DimensionGateError, "timestamp"):
                    GATE.verify_official_world(world, plan, previous)


class EntrypointContractTests(unittest.TestCase):
    def test_bbb_clean_stop_rejects_error_and_panic_logs(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            log = Path(raw) / "bbb.log"
            process = mock.Mock()
            process.wait.return_value = 0
            completed = SimpleNamespace(returncode=0, stdout=b"", stderr=b"")
            for complaint in (
                '{"level":"ERROR","message":"save worker failed"}',
                "thread 'main' panicked at server tick",
                "Exception: uncaught server failure",
                "FATAL error: tick worker exited",
            ):
                log.write_text(
                    '{"event":"world.saved"}\n' + complaint + '\n{"event":"server.stopped"}\n',
                    encoding="utf-8",
                )
                with (
                    self.subTest(complaint=complaint),
                    mock.patch.object(GATE.subprocess, "run", return_value=completed),
                    self.assertRaisesRegex(GATE.DimensionGateError, "logged errors"),
                ):
                    GATE.stop_bbb(Path("bbb-admin"), "http://mcp", process, log, 1)
            self.assertEqual(
                GATE._bbb_log_complaints(
                    '{"event":"command.executed","result":0,"message":"Unknown command"}'
                ),
                [],
            )

    def test_operator_fixture_writer_has_a_bounded_subprocess(self) -> None:
        completed = SimpleNamespace(returncode=0, stdout=b"", stderr=b"")
        with (
            tempfile.TemporaryDirectory() as raw,
            mock.patch.object(GATE.subprocess, "run", return_value=completed) as run,
        ):
            GATE.write_operator(Path(raw), Path(raw) / "admin")
        self.assertEqual(run.call_args.kwargs["timeout"], 60)

    def test_mock_live_path_orders_both_servers_and_writes_the_observation(self) -> None:
        events: list[str] = []

        class Input:
            def write(self, payload: bytes) -> None:
                events.append("official-command:" + payload.decode().strip())

            def flush(self) -> None:
                pass

        class Process:
            def __init__(self, *, official: bool) -> None:
                self.official = official
                self.stdin = Input() if official else None

            def wait(self, timeout: float) -> int:
                if self.official:
                    events.append("official-clean-stop")
                return 0

            def poll(self) -> None:
                return None

            def terminate(self) -> None:
                raise AssertionError("happy path must stop cleanly")

            def kill(self) -> None:
                raise AssertionError("happy path must stop cleanly")

        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            artifact = root / "artifact"
            properties = root / "shared.properties"
            properties.write_text("online-mode=false\n", encoding="utf-8")
            server_jar = root / "server.jar"
            server_jar.write_bytes(b"managed server")
            args = SimpleNamespace(
                repo_root=root,
                artifact_dir=artifact,
                client_bin=root / "bbb-client",
                server_bin=root / "bbb-server",
                admin_bin=root / "bbb-admin",
                java_bin=root / "java",
                server_jar=server_jar,
                shared_properties=properties,
                timeout=1,
                gamemode="creative",
                difficulty="peaceful",
            )
            keys = list(dict.fromkeys(GATE.EXPECTED_SEQUENCE))
            plan = [
                {
                    "key": key,
                    "min_y": 0,
                    "height": 16,
                    "x": index * 512 + 8,
                    "y": 8,
                    "z": index * 512 + 8,
                }
                for index, key in enumerate(keys)
            ]
            bbb_facts = {
                "written_dimension_keys": keys,
                "dimensions": [{"key": key, "published_at": 10} for key in keys],
            }
            official_facts = {"saved_dimension_keys": keys}
            processes = iter((Process(official=False), Process(official=True)))

            def popen(command: list[str], **_kwargs: object) -> Process:
                process = next(processes)
                events.append("official-start" if process.official else "bbb-start")
                return process

            probe_count = 0

            def run_probe(*_args: object, **_kwargs: object) -> dict:
                nonlocal probe_count
                side = "bbb" if probe_count == 0 else "official"
                probe_count += 1
                events.append(f"{side}-probe")
                return {"side": side}

            def wait_log(
                path: Path,
                marker: str,
                _process: Process,
                _timeout: float,
                *,
                after: int = 0,
            ) -> None:
                events.append(f"wait:{marker}:{after}")
                if marker == "Saved the game":
                    path.write_text("All dimensions are saved\nSaved the game\n", encoding="utf-8")

            def command_report(*_args: object) -> dict:
                command = str(_args[-1])
                events.append("bbb-command:" + command)
                return {"result": 1}

            def send_until_log(
                _process: Process,
                _log: Path,
                _command: str,
                marker: str,
                _timeout: float,
            ) -> None:
                events.append("official-readback:" + marker)

            def atomic_write(path: Path, _payload: bytes) -> None:
                events.append("atomic:" + path.resolve().relative_to(root.resolve()).as_posix())

            compared = {
                "configuration": {"registry_count": 28},
                "transitions": {"sequence": list(GATE.EXPECTED_SEQUENCE)},
            }
            with (
                mock.patch.dict(os.environ, {"BBB_OFFICIAL_CAPTURE_ACCEPT_EULA": "1"}),
                mock.patch.object(
                    GATE,
                    "write_operator",
                    side_effect=lambda _root, directory: events.append(
                        "operator:" + directory.name
                    ),
                ),
                mock.patch.object(GATE, "free_port", side_effect=(25565, 25566, 25567)),
                mock.patch.object(GATE.subprocess, "Popen", side_effect=popen),
                mock.patch.object(GATE, "wait_log", side_effect=wait_log),
                mock.patch.object(GATE, "run_probe", side_effect=run_probe),
                mock.patch.object(GATE, "marker_plan", return_value=plan),
                mock.patch.object(GATE, "command_report", side_effect=command_report),
                mock.patch.object(
                    GATE,
                    "stop_bbb",
                    side_effect=lambda *_args: events.append("bbb-clean-stop"),
                ),
                mock.patch.object(
                    GATE,
                    "inspect_bbb_client_world",
                    side_effect=lambda *_args: events.append("bbb-verify") or bbb_facts,
                ),
                mock.patch.object(GATE, "send_until_log", side_effect=send_until_log),
                mock.patch.object(
                    GATE,
                    "compare_probes",
                    side_effect=lambda *_args: events.append("probe-compare") or compared,
                ),
                mock.patch.object(
                    GATE,
                    "verify_official_world",
                    side_effect=lambda *_args: events.append("official-verify") or official_facts,
                ),
                mock.patch.object(GATE, "atomic_write", side_effect=atomic_write),
            ):
                result = GATE.live(args)

            bbb_commands = [event for event in events if event.startswith("bbb-command:")]
            self.assertEqual(len(bbb_commands), 6)
            for index, key in enumerate(keys):
                self.assertIn(f"execute in {key} run forceload add", bbb_commands[index * 2])
                self.assertIn("minecraft:diamond_block", bbb_commands[index * 2 + 1])
                readback = events.index(f"official-readback:bbb-dimension-loaded-{index}")
                mutation = next(
                    offset
                    for offset, event in enumerate(events)
                    if event.startswith(f"official-command:execute in {key} run setblock")
                )
                self.assertLess(readback, mutation)
            self.assertLess(events.index("bbb-probe"), events.index("bbb-clean-stop"))
            self.assertLess(events.index(bbb_commands[-1]), events.index("bbb-clean-stop"))
            self.assertLess(events.index("bbb-clean-stop"), events.index("official-start"))
            self.assertLess(events.index("official-start"), events.index("official-probe"))
            self.assertLess(
                events.index("official-readback:official-dimension-mutated-2"),
                events.index("official-probe"),
            )
            self.assertLess(events.index("official-probe"), events.index("probe-compare"))
            flush = events.index("official-command:save-all flush")
            stop = events.index("official-command:stop")
            self.assertLess(events.index("probe-compare"), flush)
            self.assertLess(flush, stop)
            self.assertLess(stop, events.index("official-clean-stop"))
            self.assertLess(events.index("official-clean-stop"), events.index("official-verify"))
            self.assertEqual(events[-1], "atomic:artifact/observation.json")
            self.assertEqual(result["artifact_dir"], str(artifact.resolve()))
            self.assertFalse((root / "docs/evidence").exists())

    @unittest.skipUnless(os.name == "posix", "POSIX shell harness")
    def test_mode_and_eula_fail_before_artifact_writes(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            artifact = Path(raw) / "artifact"
            environment = {
                "PATH": "/usr/bin:/bin",
                "BBB_SERVER_DIMENSION_RUNTIME_ARTIFACT_DIR": str(artifact),
            }
            invalid = subprocess.run(
                ["sh", str(SCRIPT), "other"], env=environment, capture_output=True, text=True
            )
            self.assertEqual(invalid.returncode, 2)
            missing = subprocess.run(
                ["sh", str(SCRIPT), "live"], env=environment, capture_output=True, text=True
            )
            self.assertEqual(missing.returncode, 2)
            self.assertIn("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA", missing.stderr)
            self.assertFalse(artifact.exists())

    def test_python_live_cannot_bypass_the_eula_gate(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            artifact = Path(raw) / "artifact"
            args = GATE.parse_args(
                [
                    "live",
                    "--artifact-dir",
                    str(artifact),
                    "--client-bin",
                    "/missing/client",
                    "--server-bin",
                    "/missing/server",
                    "--admin-bin",
                    "/missing/admin",
                    "--java-bin",
                    "/missing/java",
                    "--server-jar",
                    "/missing/server.jar",
                    "--shared-properties",
                    "/missing/server.properties",
                ]
            )
            with (
                mock.patch.dict(os.environ, {}, clear=True),
                self.assertRaisesRegex(GATE.DimensionGateError, "BBB_OFFICIAL_CAPTURE_ACCEPT_EULA"),
            ):
                GATE.live(args)
            self.assertFalse(artifact.exists())

    def test_live_gate_order_is_fail_closed(self) -> None:
        shell = SCRIPT.read_text(encoding="utf-8")
        self.assertLess(
            shell.index('"$1" != live'), shell.index("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA")
        )
        eula = shell.index("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA")
        managed = shell.index("vanilla_root=")
        freshness = shell.index("require_fresh_bin")
        artifact = shell.index("artifact_dir=")
        self.assertLess(eula, managed)
        self.assertLess(managed, freshness)
        self.assertLess(freshness, artifact)
        self.assertLess(eula, shell.index("mktemp -d"))
        body = (ROOT / "scripts/server_dimension_runtime_gate.py").read_text(encoding="utf-8")
        readback = body.index("readback = (")
        mutation = body.index("mutation = (")
        cross_second = body.index("while int(time.time()) <= published_at")
        flush = body.index('official_process.stdin.write(b"save-all flush')
        stop = body.index('official_process.stdin.write(b"stop')
        self.assertLess(readback, mutation)
        self.assertLess(mutation, cross_second)
        self.assertLess(cross_second, flush)
        self.assertLess(body.index("flush_output =", flush), stop)
        self.assertLess(flush, stop)

    def test_probe_forces_full_registry_payload_without_a_vanilla_root(self) -> None:
        body = (ROOT / "scripts/server_dimension_runtime_gate.py").read_text(encoding="utf-8")
        self.assertIn('"--probe-server-dimension-transitions"', body)
        self.assertNotIn('"--vanilla-root"', body)


if __name__ == "__main__":
    unittest.main()
