import importlib.util
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "capture_official_world", ROOT / "scripts" / "capture_official_world.py"
)
assert SPEC and SPEC.loader
CAPTURE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CAPTURE)

SCRIPT = ROOT / "scripts" / "capture-official-world-26.1.sh"


def make_world(root: Path) -> Path:
    """Create a minimal but structurally complete synthetic 26.1 world save.

    Vanilla 26.1 stores every dimension under `dimensions/<ns>/<name>`
    (`DimensionType.getStorageFolder`) and playerdata under `players/data`
    (`LevelResource.PLAYER_DATA_DIR`).
    """
    world = root / "world"
    overworld = world / "dimensions" / "minecraft" / "overworld"
    (overworld / "region").mkdir(parents=True)
    (world / "players" / "data").mkdir(parents=True)
    (world / "data").mkdir()
    (world / "level.dat").write_bytes(b"level")
    (world / "level.dat_old").write_bytes(b"old")
    (overworld / "region" / "r.0.0.mca").write_bytes(b"region-zero")
    (overworld / "region" / "r.-1.0.mca").write_bytes(b"region-neg")
    (world / "players" / "data" / "7f3f.dat").write_bytes(b"player")
    (world / "data" / "raids.dat").write_bytes(b"raids")
    return world


def make_tool(root: Path) -> tuple[Path, Path]:
    java = root / "java"
    jar = root / "server-26.1.jar"
    java.write_bytes(b"java-binary")
    jar.write_bytes(b"server-jar")
    return java, jar


def capture_receipt(world: Path, java: Path, jar: Path) -> dict:
    return CAPTURE.build_receipt(
        world_dir=world,
        java_bin=java,
        server_jar=jar,
        captured_at="2026-08-15T00:00:00Z",
    )


class ManifestTests(unittest.TestCase):
    def test_manifest_is_sorted_and_complete(self):
        with tempfile.TemporaryDirectory() as directory:
            world = make_world(Path(directory))
            manifest = CAPTURE.build_manifest(world)
            paths = [entry["path"] for entry in manifest]
            self.assertEqual(paths, sorted(paths))
            self.assertEqual(len(set(paths)), len(paths))
            self.assertIn("level.dat", paths)
            self.assertIn("dimensions/minecraft/overworld/region/r.0.0.mca", paths)
            self.assertIn("players/data/7f3f.dat", paths)
            self.assertIn("data/raids.dat", paths)
            entry = next(e for e in manifest if e["path"] == "level.dat")
            self.assertEqual(entry["size"], len(b"level"))
            self.assertEqual(
                entry["sha256"],
                CAPTURE.sha256_file(world / "level.dat"),
            )

    def test_exclusions(self):
        with tempfile.TemporaryDirectory() as directory:
            world = make_world(Path(directory))
            region = world / "dimensions" / "minecraft" / "overworld" / "region"
            (world / "session.lock").write_bytes(b"lock")
            (region / "r.0.0.mca.tmp").write_bytes(b"partial")
            (world / "players" / "data" / "7f3f.dat.tmp").write_bytes(b"partial")
            paths = [e["path"] for e in CAPTURE.build_manifest(world)]
            self.assertNotIn("session.lock", paths)
            self.assertFalse(any(path.endswith(".tmp") for path in paths))

    def test_missing_level_dat_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            world = make_world(Path(directory))
            (world / "level.dat").unlink()
            with self.assertRaisesRegex(ValueError, "level.dat"):
                CAPTURE.check_world_structure(world, CAPTURE.build_manifest(world))

    def test_missing_region_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            world = make_world(Path(directory))
            region = world / "dimensions" / "minecraft" / "overworld" / "region"
            for candidate in region.glob("*.mca"):
                candidate.unlink()
            with self.assertRaisesRegex(ValueError, "region"):
                CAPTURE.check_world_structure(world, CAPTURE.build_manifest(world))

    def test_missing_playerdata_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            world = make_world(Path(directory))
            (world / "players" / "data" / "7f3f.dat").unlink()
            with self.assertRaisesRegex(ValueError, "players/data"):
                CAPTURE.check_world_structure(world, CAPTURE.build_manifest(world))

    def test_missing_data_dir_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            import shutil

            world = make_world(Path(directory))
            shutil.rmtree(world / "data")
            with self.assertRaisesRegex(ValueError, "data/"):
                CAPTURE.check_world_structure(world, CAPTURE.build_manifest(world))

    def test_dimension_dirs_default_and_nether(self):
        with tempfile.TemporaryDirectory() as directory:
            world = make_world(Path(directory))
            self.assertEqual(CAPTURE.dimension_dirs(world), ["overworld"])
            (world / "dimensions" / "minecraft" / "the_nether" / "region").mkdir(
                parents=True
            )
            self.assertEqual(
                CAPTURE.dimension_dirs(world), ["overworld", "the_nether"]
            )


class ReceiptTests(unittest.TestCase):
    def test_capture_then_verify_round_trip(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            world = make_world(root)
            java, jar = make_tool(root)
            receipt = capture_receipt(world, java, jar)
            self.assertEqual(receipt["schema_version"], 1)
            self.assertEqual(receipt["vanilla_version"], "26.1")
            self.assertEqual(receipt["world"]["seed"], "bbb-official-capture-26.1")
            self.assertEqual(
                receipt["world_identity"],
                CAPTURE.manifest_identity(receipt["files"]),
            )
            CAPTURE.verify_world(world, receipt)

    def test_verify_detects_changed_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            world = make_world(root)
            java, jar = make_tool(root)
            receipt = capture_receipt(world, java, jar)
            target = world / "data" / "raids.dat"
            flipped = bytes([target.read_bytes()[0] ^ 0xFF])
            target.write_bytes(flipped)
            with self.assertRaisesRegex(ValueError, "changed"):
                CAPTURE.verify_world(world, receipt)

    def test_verify_detects_removed_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            world = make_world(root)
            java, jar = make_tool(root)
            receipt = capture_receipt(world, java, jar)
            (world / "level.dat_old").unlink()
            with self.assertRaisesRegex(ValueError, "missing"):
                CAPTURE.verify_world(world, receipt)

    def test_atomic_write_leaves_no_tmp_and_valid_json(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            world = make_world(root)
            java, jar = make_tool(root)
            receipt = capture_receipt(world, java, jar)
            output = root / "nested" / "receipt.json"
            CAPTURE.atomic_write_json(output, receipt)
            self.assertFalse((root / "nested" / "receipt.json.tmp").exists())
            loaded = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(loaded, receipt)
            self.assertTrue(output.read_bytes().endswith(b"\n"))
            CAPTURE.load_receipt(output)


class SchemaValidationTests(unittest.TestCase):
    def valid_receipt(self) -> dict:
        self._tmp = tempfile.TemporaryDirectory()
        root = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        return capture_receipt(make_world(root), *make_tool(root))

    def test_accepts_valid_receipt(self):
        CAPTURE.validate_capture_receipt(self.valid_receipt())

    def test_rejects_extra_key(self):
        receipt = self.valid_receipt()
        receipt["extra"] = True
        with self.assertRaisesRegex(ValueError, "keys drifted"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_missing_key(self):
        receipt = self.valid_receipt()
        del receipt["dimensions"]
        with self.assertRaisesRegex(ValueError, "keys drifted"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_wrong_schema_version(self):
        receipt = self.valid_receipt()
        receipt["schema_version"] = 2
        with self.assertRaisesRegex(ValueError, "schema_version"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_wrong_vanilla_version(self):
        receipt = self.valid_receipt()
        receipt["vanilla_version"] = "1.21.1"
        with self.assertRaisesRegex(ValueError, "vanilla_version"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_bad_hash_length(self):
        receipt = self.valid_receipt()
        receipt["tool"]["server_jar_sha256"] = "abc123"
        with self.assertRaisesRegex(ValueError, "server_jar_sha256"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_unsorted_files(self):
        receipt = self.valid_receipt()
        files = receipt["files"]
        self.assertGreaterEqual(len(files), 2)
        files[0], files[1] = files[1], files[0]
        receipt["world_identity"] = CAPTURE.manifest_identity(files)
        with self.assertRaisesRegex(ValueError, "sorted"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_wrong_world_identity(self):
        receipt = self.valid_receipt()
        receipt["world_identity"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "world_identity"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_duplicate_paths(self):
        receipt = self.valid_receipt()
        receipt["files"].append(dict(receipt["files"][-1]))
        receipt["world_identity"] = CAPTURE.manifest_identity(receipt["files"])
        with self.assertRaisesRegex(ValueError, "sorted|unique"):
            CAPTURE.validate_capture_receipt(receipt)

    def test_rejects_bad_captured_at(self):
        receipt = self.valid_receipt()
        receipt["captured_at"] = "2026-08-15 00:00:00"
        with self.assertRaisesRegex(ValueError, "captured_at"):
            CAPTURE.validate_capture_receipt(receipt)


class ShellScriptTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "posix", "POSIX shell harness")
    def test_shell_syntax(self):
        subprocess.run(["sh", "-n", str(SCRIPT)], check=True)

    @unittest.skipUnless(os.name == "posix", "POSIX shell harness")
    def test_eula_gate_exits_2_without_writing_eula(self):
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run(
                ["sh", str(SCRIPT)],
                env={
                    "PATH": "/usr/bin:/bin",
                    "BBB_OFFICIAL_CAPTURE_ARTIFACT_DIR": directory,
                },
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("BBB_OFFICIAL_CAPTURE_ACCEPT_EULA", result.stderr)
            self.assertFalse((Path(directory) / "server" / "eula.txt").exists())

    def test_every_caller_sends_the_receipt_into_its_own_artifact_directory(self):
        """A harness that only wants the world may not re-pin the repository's.

        The capture script's second job is to rewrite
        `scripts/data/server-official-capture-26.1.json`, and every capture
        writes a different one because the official server is not
        deterministic across runs. A harness that calls it for the world alone
        and leaves `BBB_OFFICIAL_CAPTURE_RECEIPT` at its default therefore
        silently replaces a checked-in source file -- one the parity acceptance
        plan fingerprints -- on every run. Both current callers redirect it;
        this is what keeps a third from forgetting.
        """
        callers = sorted(
            [
                *(ROOT / "scripts").glob("*.sh"),
                *(ROOT / "scripts").glob("*.py"),
            ],
            key=lambda p: p.name,
        )
        callers = [
            path
            for path in callers
            if path.name != SCRIPT.name and SCRIPT.name in path.read_text(encoding="utf-8")
        ]
        self.assertEqual(
            [path.name for path in callers],
            [
                "server-round-trip-gate-26.1.sh",
                "server-world-oracle-26.1.sh",
                "server_release_gate.py",
            ],
            "a new caller of the capture script has to be checked here too",
        )
        for path in callers:
            self.assertIn(
                "BBB_OFFICIAL_CAPTURE_RECEIPT",
                path.read_text(encoding="utf-8"),
                f"{path.name} calls the capture script without redirecting its receipt",
            )


if __name__ == "__main__":
    unittest.main()
