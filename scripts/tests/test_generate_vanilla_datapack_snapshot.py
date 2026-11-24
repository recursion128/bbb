from __future__ import annotations

import gzip
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

import pytest
from bbb_tools.vanilla.workspace import VanillaWorkspace

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "generate_vanilla_datapack_snapshot",
    SCRIPTS / "generate-vanilla-datapack-snapshot.py",
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)

REPO_ROOT = Path(__file__).resolve().parents[2]
ARTIFACT = (
    REPO_ROOT / "crates/bbb-server-datapack/data/vanilla_datapack_snapshot_26_1.bin.gz"
)


def vanilla_source_root() -> Path:
    return VanillaWorkspace.resolve("26.1").require_prepared().sources


def expected_tree_records(source_root: Path) -> dict[str, bytes]:
    """Independently walk the declared vanilla directories into path -> bytes."""
    data_root = source_root / "data/minecraft"
    expected: dict[str, bytes] = {}
    for directory in GENERATOR.DECLARED_DIRECTORIES:
        declared = data_root / directory
        if not declared.is_dir():
            continue
        for path in declared.rglob("*"):
            if path.is_file():
                expected[path.relative_to(data_root).as_posix()] = path.read_bytes()
    return expected


class VanillaDatapackSnapshotFormatTests(unittest.TestCase):
    def test_committed_artifact_format_guards(self) -> None:
        blob = ARTIFACT.read_bytes()
        self.assertEqual(blob[4:8], b"\x00\x00\x00\x00", "gzip mtime must be zero")
        self.assertFalse(blob[3] & 0x08, "gzip stream must not carry a filename")

        records = GENERATOR.decode_records(blob)
        self.assertGreater(len(records), 0)
        paths = [path for path, _ in records]
        self.assertEqual(
            paths,
            sorted(paths, key=lambda path: path.encode("utf-8")),
            "record paths must be sorted by path bytes",
        )
        self.assertEqual(len(paths), len(set(paths)), "record paths must be unique")
        for path, data in records:
            self.assertTrue(path, "record path must be non-empty")
            self.assertFalse(path.startswith("/"), f"absolute record path: {path}")
            self.assertNotIn("\\", path, f"non-POSIX separator in {path}")
            self.assertNotIn("..", path.split("/"), f"parent component in {path}")
            self.assertIsInstance(data, bytes)
        top_level = {path.split("/", 1)[0] for path in paths}
        self.assertFalse(
            top_level & GENERATOR.FORBIDDEN_DIRECTORIES,
            "forbidden directories leaked into the snapshot",
        )

    def test_decode_rejects_truncated_payload(self) -> None:
        blob = GENERATOR.compress_payload(
            GENERATOR.encode_records([("a/b.json", b"payload")])
        )
        records = GENERATOR.decode_records(blob)
        self.assertEqual(records, [("a/b.json", b"payload")])

        payload = gzip.decompress(blob)
        truncated = GENERATOR.compress_payload(payload[:-3])
        with self.assertRaisesRegex(ValueError, "truncated data"):
            GENERATOR.decode_records(truncated)

    def test_missing_declared_directory_fails_loudly(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            data_root = Path(directory) / "data/minecraft"
            for name in GENERATOR.REQUIRED_DIRECTORIES:
                if name != "worldgen":
                    (data_root / name).mkdir(parents=True, exist_ok=True)
            with self.assertRaisesRegex(ValueError, "worldgen"):
                GENERATOR.collect_records(Path(directory))

    def test_forbidden_directory_cannot_be_declared(self) -> None:
        declared = set(GENERATOR.DECLARED_DIRECTORIES)
        self.assertFalse(declared & GENERATOR.FORBIDDEN_DIRECTORIES)


@pytest.mark.vanilla
class VanillaDatapackSnapshotGoldenTests(unittest.TestCase):
    def test_generation_is_deterministic(self) -> None:
        source_root = vanilla_source_root()
        _, first = GENERATOR.build_blob(source_root)
        _, second = GENERATOR.build_blob(source_root)
        self.assertEqual(first, second)

    def test_blob_matches_vanilla_tree_exactly(self) -> None:
        source_root = vanilla_source_root()
        records, _ = GENERATOR.build_blob(source_root)
        decoded = dict(records)
        self.assertEqual(len(decoded), len(records), "duplicate record paths")
        expected = expected_tree_records(source_root)
        missing = sorted(set(expected) - set(decoded))
        extra = sorted(set(decoded) - set(expected))
        self.assertEqual(missing, [], "records missing from the blob")
        self.assertEqual(extra, [], "unexpected records in the blob")
        changed = sorted(
            path for path in expected if expected[path] != decoded[path]
        )
        self.assertEqual(changed, [], "record content drift")

    def test_check_passes_against_committed_artifact(self) -> None:
        self.assertEqual(GENERATOR.main(["--check"]), 0)


if __name__ == "__main__":
    unittest.main()
