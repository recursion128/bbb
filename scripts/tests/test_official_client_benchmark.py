import hashlib
import importlib.util
import json
import os
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import zipfile
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "official_client_benchmark.py"
SPEC = importlib.util.spec_from_file_location("official_client_benchmark", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
benchmark = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = benchmark
SPEC.loader.exec_module(benchmark)


def sha1(payload: bytes) -> str:
    return hashlib.sha1(payload).hexdigest()


class DistributionTests(unittest.TestCase):
    def write_distribution(self, root: Path) -> tuple[Path, Path]:
        downloads = root / "downloads" / "26.1"
        libraries = downloads / "libraries"
        assets = downloads / "assets"
        downloads.mkdir(parents=True)
        java = root / "java"
        java.write_text("#!/bin/sh\n", encoding="utf-8")
        java.chmod(0o755)
        client = b"client"
        client_path = downloads / "client-26.1.jar"
        client_path.write_bytes(client)
        ordinary = b"ordinary"
        native = b"native-archive"
        ordinary_path = libraries / "example/ordinary.jar"
        native_path = libraries / "example/natives-linux.jar"
        ordinary_path.parent.mkdir(parents=True)
        ordinary_path.write_bytes(ordinary)
        native_path.write_bytes(native)
        sound = b"sound"
        sound_hash = sha1(sound)
        object_path = assets / "objects" / sound_hash[:2] / sound_hash
        object_path.parent.mkdir(parents=True)
        object_path.write_bytes(sound)
        index = {
            "objects": {
                "minecraft/sounds/test.ogg": {
                    "hash": sound_hash,
                    "size": len(sound),
                }
            }
        }
        index_payload = json.dumps(index).encode()
        index_path = assets / "indexes/fixture.json"
        index_path.parent.mkdir(parents=True)
        index_path.write_bytes(index_payload)
        metadata = {
            "id": "26.1",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {
                "client": {"sha1": sha1(client), "size": len(client)},
                "server": {"sha1": sha1(b"server"), "size": len(b"server")},
            },
            "libraries": [
                {
                    "name": "example:ordinary:1",
                    "downloads": {
                        "artifact": {
                            "path": "example/ordinary.jar",
                            "sha1": sha1(ordinary),
                            "size": len(ordinary),
                        }
                    },
                },
                {
                    "name": "example:native:1:natives-linux",
                    "rules": [{"action": "allow", "os": {"name": "linux"}}],
                    "downloads": {
                        "artifact": {
                            "path": "example/natives-linux.jar",
                            "sha1": sha1(native),
                            "size": len(native),
                        }
                    },
                },
                {
                    "name": "example:windows:1",
                    "rules": [{"action": "allow", "os": {"name": "windows"}}],
                    "downloads": {
                        "artifact": {
                            "path": "../outside.jar",
                            "sha1": "",
                            "size": 0,
                        }
                    },
                },
            ],
            "assetIndex": {
                "id": "fixture",
                "sha1": sha1(index_payload),
                "size": len(index_payload),
                "totalSize": len(sound),
            },
        }
        metadata_payload = json.dumps(metadata).encode()
        metadata_path = downloads / "26.1.json"
        metadata_path.write_bytes(metadata_payload)
        (downloads / "server-26.1.jar").write_bytes(b"server")
        (downloads / "version_manifest_v2.json").write_text(
            json.dumps(
                {
                    "versions": [
                        {"id": "26.1", "sha1": sha1(metadata_payload)}
                    ]
                }
            ),
            encoding="utf-8",
        )
        return root, java

    def test_distribution_applies_rules_and_validates_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            root, java = self.write_distribution(Path(directory))
            audit = benchmark.audit_distribution(root, java_path=java)
            self.assertEqual(len(audit.libraries), 1)
            self.assertEqual(len(audit.native_classifiers), 1)
            self.assertEqual(audit.asset_objects, 1)
            self.assertEqual(audit.asset_object_bytes, 5)
            server = benchmark.validate_server_jar(root)
            self.assertEqual(server.sha1, sha1(b"server"))

    def test_distribution_rejects_hash_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root, java = self.write_distribution(Path(directory))
            (root / "downloads/26.1/client-26.1.jar").write_bytes(b"wrong!")
            with self.assertRaisesRegex(
                benchmark.BenchmarkFailure, "client jar SHA-1 mismatch"
            ):
                benchmark.audit_distribution(root, java_path=java)

    def test_library_rule_order_and_feature_matching(self):
        library = {
            "rules": [
                {"action": "allow", "os": {"name": "linux"}},
                {
                    "action": "disallow",
                    "os": {"name": "linux"},
                    "features": {"demo": True},
                },
            ]
        }
        self.assertTrue(benchmark.library_allowed(library, features={"demo": False}))
        self.assertFalse(benchmark.library_allowed(library, features={"demo": True}))


class NativeExtractionTests(unittest.TestCase):
    def write_zip(self, path: Path, entries: dict[str, bytes]) -> None:
        with zipfile.ZipFile(path, "w") as archive:
            for name, payload in entries.items():
                archive.writestr(name, payload)

    def test_extracts_only_native_shared_libraries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "natives.jar"
            self.write_zip(
                archive,
                {
                    "linux/x64/org/lwjgl/liblwjgl.so": b"native",
                    "META-INF/MANIFEST.MF": b"ignored",
                },
            )
            extracted = benchmark.safe_extract_natives([archive], root / "out")
            self.assertEqual([Path(entry.path).name for entry in extracted], ["liblwjgl.so"])
            self.assertEqual((root / "out/liblwjgl.so").read_bytes(), b"native")

    def test_rejects_path_escape_even_for_skipped_members(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "escape.jar"
            self.write_zip(archive, {"../escape.txt": b"bad", "libok.so": b"ok"})
            with self.assertRaisesRegex(
                benchmark.BenchmarkFailure, "escapes target"
            ):
                benchmark.safe_extract_natives([archive], root / "out")

    def test_rejects_native_basename_collision(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = root / "first.jar"
            second = root / "second.jar"
            self.write_zip(first, {"a/libsame.so": b"one"})
            self.write_zip(second, {"b/libsame.so": b"two"})
            with self.assertRaisesRegex(
                benchmark.BenchmarkFailure, "basename collision"
            ):
                benchmark.safe_extract_natives([first, second], root / "out")


class LaunchPlanTests(unittest.TestCase):
    def audit(self) -> object:
        verified = benchmark.VerifiedFile("/tmp/client.jar", "a" * 40, 1)
        return benchmark.DistributionAudit(
            schema=1,
            version="26.1",
            metadata=verified,
            client=verified,
            libraries=(
                benchmark.VerifiedFile("/tmp/library.jar", "b" * 40, 1),
            ),
            native_classifiers=(),
            asset_index=verified,
            asset_objects=1,
            asset_object_bytes=1,
            asset_index_id="30",
            main_class="net.minecraft.client.main.Main",
            resource_hash="c" * 64,
        )

    def test_offline_launch_arguments_are_fixed_and_credential_free(self):
        plan = benchmark.build_vanilla_launch_plan(
            self.audit(),
            java_path=Path("/tmp/java"),
            game_dir=Path("/tmp/game"),
            assets_dir=Path("/tmp/assets"),
            natives_dir=Path("/tmp/natives"),
            server_port=25565,
        )
        argv = list(plan.argv)
        self.assertIn("--offlineDeveloperMode", argv)
        self.assertIn("-DMC_DEBUG_ENABLED", argv)
        self.assertIn("-DMC_DEBUG_PREFER_WAYLAND", argv)
        self.assertEqual(argv[argv.index("--username") + 1], "bbb-benchmark")
        self.assertEqual(argv[argv.index("--accessToken") + 1], "offline")
        self.assertNotIn("--uuid", argv)
        self.assertEqual(argv[argv.index("--width") + 1], "1280")
        self.assertEqual(argv[argv.index("--height") + 1], "720")
        self.assertEqual(
            argv[argv.index("--quickPlayMultiplayer") + 1],
            "127.0.0.1:25565",
        )
        self.assertNotIn("offline", benchmark.sanitized_argv(argv))

    def test_options_pin_equal_load_settings(self):
        options = benchmark.vanilla_options()
        self.assertEqual(options["version"], "4786")
        self.assertEqual(options["renderDistance"], "12")
        self.assertEqual(options["simulationDistance"], "12")
        self.assertEqual(options["graphicsPreset"], '"custom"')
        self.assertEqual(options["ao"], "true")
        self.assertEqual(options["biomeBlendRadius"], "2")
        self.assertEqual(options["prioritizeChunkUpdates"], "1")
        self.assertEqual(options["renderClouds"], '"true"')
        self.assertEqual(options["cloudRange"], "64")
        self.assertEqual(options["particles"], "0")
        self.assertEqual(options["mipmapLevels"], "4")
        self.assertEqual(options["cutoutLeaves"], "true")
        self.assertEqual(options["entityShadows"], "true")
        self.assertEqual(options["improvedTransparency"], "false")
        self.assertEqual(options["weatherRadius"], "10")
        self.assertEqual(options["maxAnisotropyBit"], "1")
        self.assertEqual(options["textureFiltering"], "1")
        self.assertEqual(options["guiScale"], "1")
        self.assertNotIn("hideGui", options)
        self.assertEqual(options["bobView"], "true")
        self.assertEqual(options["soundCategory_master"], "0.0")
        self.assertEqual(options["enableVsync"], "false")
        self.assertEqual(options["maxFps"], "260")
        self.assertEqual(options["onboardAccessibility"], "false")
        self.assertEqual(options["menuBackgroundBlurriness"], "0")

        with tempfile.TemporaryDirectory() as directory:
            path = benchmark.write_vanilla_options(Path(directory))
            loaded = path.read_text(encoding="utf-8")
            path.write_text(loaded, encoding="utf-8")
            audit = benchmark.audit_vanilla_options(path)
            self.assertEqual(audit["values"]["renderDistance"], 12)
            self.assertEqual(audit["values"]["renderClouds"], "true")

            path.write_text(
                loaded.replace("simulationDistance:12", "simulationDistance:8"),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                benchmark.BenchmarkFailure, "simulationDistance"
            ):
                benchmark.audit_vanilla_options(path)

    def test_bbb_launch_arguments_pin_the_same_external_settings(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "bbb-native"
            binary.write_text("#!/bin/sh\n", encoding="utf-8")
            binary.chmod(0o755)
            argv = benchmark.build_bbb_launch_argv(
                binary=binary,
                server_port=25565,
                control_port=25566,
            )
        self.assertEqual(argv[argv.index("--username") + 1], "bbb-benchmark")
        self.assertEqual(argv[argv.index("--client-view-distance") + 1], "12")
        self.assertEqual(argv[argv.index("--render-distance") + 1], "12")
        self.assertEqual(argv[argv.index("--client-particles") + 1], "all")
        self.assertEqual(argv[argv.index("--client-vsync") + 1], "false")
        self.assertEqual(argv[argv.index("--client-framerate-limit") + 1], "260")
        settings = benchmark.equal_load_settings()
        self.assertEqual(settings["window"], {
            "width": 1280,
            "height": 720,
            "mode": "windowed",
        })
        self.assertEqual(settings["gui_scale"], 1)
        self.assertTrue(settings["view_bobbing"])
        self.assertEqual(settings["audio"], "disabled")


class ReplayTests(unittest.TestCase):
    def replay(self):
        frames = []
        tick = 0
        for segment in (
            "static_orbit",
            "preloaded_flight",
            "chunk_boundary_crossing",
        ):
            for marker in (
                "segment_start",
                "capture_start",
                "anchor",
                "capture_end",
            ):
                frames.append(
                    {
                        "server_tick": tick,
                        "x": tick * 0.25,
                        "y": 80.0,
                        "z": 0.0,
                        "yaw": 90.0,
                        "pitch": 0.0,
                        "capture_segment": segment,
                        "marker": marker,
                    }
                )
                tick += 1
        return {
            "schema": 1,
            "minecraft_version": "26.1",
            "player": "bbb-benchmark",
            "frames": frames,
        }

    def test_replay_hash_is_canonical_and_order_sensitive(self):
        replay = self.replay()
        digest = benchmark.replay_hash(replay)
        self.assertEqual(digest, benchmark.canonical_sha256(replay))
        replay["frames"][1]["x"] = 5.0
        self.assertNotEqual(digest, benchmark.replay_hash(replay))

    def test_default_replay_has_three_equal_segments_and_static_anchors(self):
        replay = benchmark.default_replay()
        self.assertEqual(len(replay["frames"]), 1440)
        for segment in replay["segments"]:
            frames = [
                frame
                for frame in replay["frames"]
                if frame["capture_segment"] == segment["id"]
            ]
            self.assertEqual(len(frames), 480)
            anchor_index = next(
                index
                for index, frame in enumerate(frames)
                if frame["marker"] == "anchor"
            )
            anchor_pose = {
                key: frames[anchor_index][key]
                for key in ("x", "y", "z", "yaw", "pitch")
            }
            for frame in frames[anchor_index : anchor_index + 40]:
                self.assertEqual(
                    {key: frame[key] for key in anchor_pose}, anchor_pose
                )

    def test_checked_in_replay_is_the_canonical_default(self):
        fixture = json.loads(
            (
                REPO_ROOT
                / "scripts/fixtures/official-client-benchmark/replay-26.1.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(fixture, benchmark.default_replay())
        self.assertEqual(
            benchmark.replay_hash(fixture),
            "b36fcf5620b93ed60fb46c2a31ef74fa6042d7e39bc4177147837fc85968a0aa",
        )

    def test_replay_rejects_noncontiguous_ticks(self):
        replay = self.replay()
        replay["frames"][2]["server_tick"] = 5
        with self.assertRaisesRegex(
            benchmark.BenchmarkFailure, "must be contiguous"
        ):
            benchmark.validate_replay(replay)

    def test_datapack_preserves_tick_position_and_rotation_order(self):
        with tempfile.TemporaryDirectory() as directory:
            replay = self.replay()
            digest = benchmark.generate_replay_datapack(
                replay, Path(directory)
            )
            tick = (
                Path(directory)
                / "data/bbb_benchmark/function/tick.mcfunction"
            ).read_text(encoding="utf-8")
            dispatch = (
                Path(directory)
                / "data/bbb_benchmark/function/dispatch/0_11.mcfunction"
            ).read_text(encoding="utf-8")
            self.assertIn(
                "run function bbb_benchmark:dispatch/0_11", tick
            )
            self.assertTrue(
                all(
                    line.startswith(
                        "execute if score #run bbb_replay_control matches 1 "
                    )
                    for line in tick.splitlines()
                )
            )
            self.assertIn(
                "matches 0 run tp @s 0 80 0 90 0", dispatch
            )
            self.assertIn(
                f"BBB_BENCHMARK_MARKER {digest} "
                "static_orbit segment_start 0 0 80 0 90 0",
                dispatch,
            )
            self.assertTrue(
                dispatch.rstrip().endswith("scoreboard players add @s bbb_replay 1")
            )
            load = (
                Path(directory)
                / "data/bbb_benchmark/function/load.mcfunction"
            ).read_text(encoding="utf-8")
            self.assertIn(
                "scoreboard players set #run bbb_replay_control 0", load
            )

    def test_server_layout_and_world_commands_are_deterministic_and_offline(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            replay = self.replay()
            digest = benchmark.write_server_layout(
                root,
                replay=replay,
                server_port=25570,
                accept_eula=True,
            )
            properties = (root / "server.properties").read_text(encoding="utf-8")
            self.assertIn("online-mode=false\n", properties)
            self.assertIn("enforce-secure-profile=false\n", properties)
            self.assertIn("level-seed=bbb-official-benchmark-26.1\n", properties)
            self.assertIn("view-distance=12\n", properties)
            self.assertEqual(
                digest,
                benchmark.replay_hash(replay),
            )
            commands = benchmark.world_preparation_commands(replay)
            self.assertIn("gamerule random_tick_speed 0", commands)
            self.assertIn("gamerule spawn_mobs false", commands)
            self.assertTrue(any(command.startswith("summon minecraft:armor_stand") for command in commands))
            self.assertEqual(commands[-3:], (
                "forceload remove all",
                "save-all flush",
                f"say BBB_BENCHMARK_WORLD_PREPARED {digest}",
            ))
            self.assertTrue(
                all(
                    "forceload add" not in command
                    or len(
                        range(
                            int(command.split()[-4]) // 16,
                            int(command.split()[-2]) // 16 + 1,
                        )
                    )
                    * len(
                        range(
                            int(command.split()[-3]) // 16,
                            int(command.split()[-1]) // 16 + 1,
                        )
                    )
                    <= 256
                    for command in commands
                )
            )

    def test_server_layout_requires_explicit_eula_acknowledgement(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(
                benchmark.BenchmarkFailure, "accept-eula"
            ):
                benchmark.write_server_layout(
                    Path(directory),
                    replay=self.replay(),
                    server_port=25570,
                    accept_eula=False,
                )

    def test_server_camera_log_replays_the_exact_marker_sequence(self):
        replay = self.replay()
        digest = benchmark.replay_hash(replay)
        lines = []
        for frame in replay["frames"]:
            lines.append(
                "[Server thread/INFO]: "
                f"BBB_BENCHMARK_MARKER {digest} "
                f"{frame['capture_segment']} {frame['marker']} "
                f"{frame['server_tick']} {frame['x']} {frame['y']} "
                f"{frame['z']} {frame['yaw']} {frame['pitch']}"
            )
        parsed = benchmark.parse_server_camera_markers("\n".join(lines), replay)
        self.assertEqual(
            [marker["server_tick"] for marker in parsed],
            [frame["server_tick"] for frame in replay["frames"]],
        )
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "server-camera.jsonl"
            evidence = benchmark.write_server_camera_log(
                output, "\n".join(lines), replay
            )
            self.assertEqual(evidence["markers"], len(replay["frames"]))
            self.assertEqual(len(evidence["sha256"]), 64)


class CaptureTests(unittest.TestCase):
    def test_mangohud_environment_starts_hidden_raw_logging(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with mock.patch.object(
                benchmark,
                "audit_mangohud",
                return_value={"name": "MangoHud", "version": "0.8.4"},
            ):
                environment, audit = benchmark.mangohud_environment(
                    package_root=root,
                    output_dir=root / "capture",
                    api="opengl",
                )
            self.assertEqual(audit["version"], "0.8.4")
            config = environment["MANGOHUD_CONFIG"]
            self.assertIn("no_display", config)
            self.assertNotIn("autostart_log", config)
            self.assertIn("log_duration=0", config)
            self.assertIn("control=bbb-benchmark-mango-%p", config)
            self.assertIn("permit_upload=0", config)
            self.assertTrue(environment["LD_PRELOAD"].endswith("libMangoHud_shim.so"))

    def test_mangohud_vulkan_environment_enables_relocated_layer(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = (
                root
                / "usr/share/vulkan/implicit_layer.d/MangoHud.x86_64.json"
            )
            manifest.parent.mkdir(parents=True)
            manifest.write_text(
                json.dumps(
                    {
                        "file_format_version": "1.0.0",
                        "layer": {
                            "name": "VK_LAYER_MANGOHUD_overlay_x86_64",
                            "library_path": "/usr/lib/libMangoHud.so",
                        },
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(
                benchmark,
                "audit_mangohud",
                return_value={"name": "MangoHud", "version": "0.8.4"},
            ):
                environment, _audit = benchmark.mangohud_environment(
                    package_root=root,
                    output_dir=root / "capture",
                    api="vulkan",
                )
            self.assertEqual(
                environment["VK_INSTANCE_LAYERS"],
                "VK_LAYER_MANGOHUD_overlay_x86_64",
            )
            self.assertEqual(
                environment["VK_LAYER_PATH"], environment["VK_ADD_LAYER_PATH"]
            )

    def test_mangohud_parser_and_frame_statistics(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "mango.csv"
            path.write_text(
                "os,cpu,gpu,ram,kernel,driver,cpuscheduler\n"
                "Linux,fixture,fixture,1,fixture,fixture,fixture\n"
                "fps,frametime,gpu_load,gpu_core_clock,gpu_temp\n"
                "100,10,50,1000,60\n"
                "50,20,70,1200,70\n"
                "25,40,90,1400,80\n",
                encoding="utf-8",
            )
            parsed = benchmark.parse_mangohud_csv(path)
            summary = benchmark.summarize_frames(parsed)
            self.assertEqual(summary.samples, 3)
            self.assertEqual(summary.p50_ms, 20.0)
            self.assertEqual(summary.one_percent_low_fps, 25.0)
            self.assertEqual(summary.hitch_counts["ge_33.333_ms"], 1)
            self.assertEqual(summary.gpu_load_mean, 70.0)

    def test_mangohud_control_starts_and_stops_logging_without_input(self):
        process_pid = os.getpid()
        address = "\0" + f"{benchmark.MANGOHUD_CONTROL_PREFIX}-{process_pid}"

        def serve(expected: bytes, response: bytes = b"") -> threading.Thread:
            ready = threading.Event()

            def run():
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
                    server.bind(address)
                    server.listen(1)
                    ready.set()
                    connection, _peer = server.accept()
                    with connection:
                        self.assertEqual(connection.recv(4096), expected)
                        if response:
                            connection.sendall(response)

            thread = threading.Thread(target=run)
            thread.start()
            self.assertTrue(ready.wait(2.0))
            return thread

        start_server = serve(b":logging=1;")
        started = benchmark.mangohud_set_logging(
            process_pid=process_pid, enabled=True
        )
        start_server.join(2.0)
        self.assertTrue(started["acknowledged"])

        stop_server = serve(b":logging=0;", b":LoggingFinished")
        stopped = benchmark.mangohud_set_logging(
            process_pid=process_pid, enabled=False
        )
        stop_server.join(2.0)
        self.assertTrue(stopped["acknowledged"])

        with tempfile.TemporaryDirectory() as directory:
            completion = Path(directory)
            fallback_ready = threading.Event()

            def close_with_files():
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as server:
                    server.bind(address)
                    server.listen(1)
                    fallback_ready.set()
                    connection, _peer = server.accept()
                    with connection:
                        self.assertEqual(connection.recv(4096), b":logging=0;")
                        (completion / "client.csv").write_text(
                            "fps,frametime\n60,16.6\n", encoding="utf-8"
                        )
                        (completion / "client_summary.csv").write_text(
                            "summary\n", encoding="utf-8"
                        )

            fallback_server = threading.Thread(target=close_with_files)
            fallback_server.start()
            self.assertTrue(fallback_ready.wait(2.0))
            fallback = benchmark.mangohud_set_logging(
                process_pid=process_pid,
                enabled=False,
                completion_dir=completion,
            )
            fallback_server.join(2.0)
            self.assertEqual(
                fallback["acknowledged_by"], "closed raw and summary files"
            )

    def test_perf_parser_normalizes_process_totals_per_presented_frame(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "perf.csv"
            path.write_text(
                "1000,msec,task-clock,1,100.00,\n"
                "2000,,cycles,1,100.00,\n"
                "3000,,instructions,1,100.00,\n"
                "4,,context-switches,1,100.00,\n"
                "5,,cpu-migrations,1,100.00,\n"
                "6,,page-faults,1,100.00,\n",
                encoding="utf-8",
            )
            parsed = benchmark.parse_perf_stat(path, presented_frames=100)
            self.assertEqual(parsed["per_presented_frame"]["cycles"], 20.0)
            self.assertIn("no CPU-frame percentiles", parsed["scope"])

    def test_perf_permission_failure_is_explicit(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "perf.txt"
            path.write_text(
                "Access to performance monitoring is limited; "
                "perf_event_paranoid setting is 4",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                benchmark.BenchmarkFailure, "kernel permission"
            ):
                benchmark.parse_perf_stat(path, presented_frames=1)

    def test_ftrace_requires_every_client_associated_event(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.txt"
            lines = [
                f"client-42 [001] ...1 1.0: {event}: pid=42 context_pid=42"
                for event in benchmark.REQUIRED_FTRACE_EVENTS
            ]
            path.write_text("\n".join(lines), encoding="utf-8")
            parsed = benchmark.parse_ftrace(path, client_pid=42)
            self.assertFalse(parsed["gpu_duration_claim_allowed"])
            self.assertTrue(all(parsed["associated_events"].values()))

    def test_invalid_run_reasons_cover_load_and_capture_contract(self):
        expected = {
            "replay_hash": "r",
            "world_hash": "w",
            "resource_hash": "a",
            "settings_hash": "s",
            "visible_content_hash": "v",
        }
        run = dict(expected)
        run.update(
            {
                "position_corrections": 1,
                "resource_failures": ["bad"],
                "resolution": [640, 480],
                "capture_dropped_frames": 2,
                "throttling": None,
                "presented_frames": 0,
                "capture_api": "unknown",
            }
        )
        reasons = benchmark.validate_run(run, expected)
        self.assertIn("position correction observed", reasons)
        self.assertIn("resource loading failure", reasons)
        self.assertIn("capture hook dropped frames", reasons)
        self.assertIn("no presented frames", reasons)

    def test_blocked_summary_never_promotes_smoke_metrics(self):
        common = {
            "replay_hash": "r",
            "world_hash": "w",
            "resource_hash": "a",
            "settings_hash": "s",
            "server_camera_log": {"sha256": "camera", "markers": 12},
            "mangohud_frame_summaries": [{"p50_ms": 10.0}],
            "mangohud_raw_logs": ["/tmp/raw.csv"],
            "client_cleanup": {"signal": "SIGKILL"},
            "screenshots": [
                {
                    "capture_segment": "static_orbit",
                    "path": "/tmp/anchor.png",
                }
            ],
        }
        preflight = {
            "blockers": [{"code": "render_node_unavailable", "detail": "fixture"}],
            "tools": {"renderdoccmd": {"path": None}},
        }
        summary = benchmark.summarize_blocked_comparison(
            preflight=preflight,
            vanilla={**common, "client": "vanilla"},
            bbb={**common, "client": "bbb"},
            visible_content_status="mismatch",
            visible_content_note="fixture mismatch",
        )
        self.assertFalse(summary["formal_comparison_complete"])
        self.assertFalse(summary["metrics_publishable"])
        self.assertTrue(all(summary["contract_matches"].values()))
        self.assertTrue(summary["server_camera_log_matches"])
        self.assertIn(
            "visible_content_mismatch",
            [blocker["code"] for blocker in summary["blockers"]],
        )


class ProcessCleanupTests(unittest.TestCase):
    def test_cleanup_terminates_the_process_group(self):
        command = [
            sys.executable,
            "-c",
            "import subprocess,time; "
            "subprocess.Popen(['sleep','60']); "
            "time.sleep(60)",
        ]
        group = benchmark.ProcessGroup(
            command,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        process = group.start()
        time.sleep(0.1)
        group.cleanup(timeout=1.0)
        self.assertIsNotNone(process.poll())
        with self.assertRaises(ProcessLookupError):
            os.killpg(process.pid, 0)


if __name__ == "__main__":
    unittest.main()
