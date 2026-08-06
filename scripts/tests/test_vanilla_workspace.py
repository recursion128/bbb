from __future__ import annotations

import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import pytest

from bbb_tools.vanilla.clean import clean_workspace
from bbb_tools.vanilla.errors import (
    DecompileFailure,
    DigestMismatch,
    DownloadFailure,
    IntegrityFailure,
    JavaIncompatible,
    UnsafeClean,
    WorkspaceLocked,
    WorkspaceMissing,
)
from bbb_tools.vanilla.integrity import tree_hash
from bbb_tools.vanilla.lockfile import load_lock
from bbb_tools.vanilla.locking import VersionLock
from bbb_tools.vanilla.prepare import _publish, _validate_cache_location
from bbb_tools.vanilla.workspace import VanillaWorkspace, default_vanilla_root


class VanillaWorkspaceTests(unittest.TestCase):
    @pytest.mark.vanilla
    def test_default_managed_workspace_is_prepared(self) -> None:
        workspace = VanillaWorkspace.resolve("26.1").require_prepared()
        self.assertTrue(workspace.sources.is_dir())
        self.assertTrue(workspace.assets.is_dir())

    def test_resolution_precedence_has_no_legacy_path_owner(self) -> None:
        environment = {
            "HOME": "/Users/tester",
            "BBB_VANILLA_ROOT": "/cache/new",
        }
        self.assertEqual(
            VanillaWorkspace.resolve("26.1", "/explicit", environ=environment).root,
            Path("/explicit"),
        )
        self.assertEqual(
            VanillaWorkspace.resolve("26.1", environ=environment).root,
            Path("/cache/new"),
        )
        environment.pop("BBB_VANILLA_ROOT")
        self.assertEqual(
            VanillaWorkspace.resolve("26.1", platform="darwin", environ=environment).root,
            Path("/Users/tester/Library/Caches/bbb/vanilla"),
        )

    def test_linux_default_honors_absolute_xdg_cache_home(self) -> None:
        self.assertEqual(
            default_vanilla_root(
                platform="linux", environ={"HOME": "/home/test", "XDG_CACHE_HOME": "/x"}
            ),
            Path("/x/bbb/vanilla"),
        )

    def test_missing_state_has_one_actionable_prepare_command(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = VanillaWorkspace(Path(directory), "26.1")
            with self.assertRaisesRegex(WorkspaceMissing, "scripts/vanilla prepare 26.1"):
                workspace.require_prepared()

    def test_tree_hash_is_order_and_mtime_independent(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "b").write_bytes(b"two")
            (root / "a").write_bytes(b"one")
            first = tree_hash(root)
            (root / "a").touch()
            self.assertEqual(tree_hash(root), first)
            (root / "a").write_bytes(b"changed")
            self.assertNotEqual(tree_hash(root), first)

    def test_clean_removes_only_one_managed_version(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = VanillaWorkspace(Path(directory) / "cache", "26.1")
            workspace.downloads.mkdir(parents=True)
            workspace.assets.mkdir(parents=True)
            workspace.state.parent.mkdir(parents=True)
            workspace.state.write_text(json.dumps({"status": "complete"}), encoding="utf-8")
            other = workspace.root / "sources" / "other" / "keep"
            other.mkdir(parents=True)
            result = clean_workspace(workspace)
            self.assertEqual(result["status"], "cleaned")
            self.assertTrue(other.is_dir())
            self.assertFalse(workspace.sources.exists())

    def test_clean_can_remove_an_interrupted_version_without_state(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = VanillaWorkspace(Path(directory) / "cache", "26.1")
            workspace.downloads.mkdir(parents=True)
            interrupted = workspace.staging / "26.1-interrupted"
            interrupted.mkdir(parents=True)
            result = clean_workspace(workspace)
            self.assertEqual(result["status"], "cleaned")
            self.assertFalse(workspace.downloads.exists())
            self.assertFalse(interrupted.exists())

    def test_clean_refuses_git_repository(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = VanillaWorkspace(Path(directory), "26.1")
            (workspace.root / ".git").mkdir()
            with self.assertRaises(UnsafeClean):
                clean_workspace(workspace)

    def test_version_lock_reports_concurrent_owner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "26.1.lock"
            with VersionLock(path):
                with self.assertRaisesRegex(WorkspaceLocked, "locked by"):
                    with VersionLock(path):
                        self.fail("the second lock unexpectedly succeeded")

    def test_publish_restores_previous_workspace_if_a_swap_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = VanillaWorkspace(root / "cache", "26.1")
            workspace.downloads.mkdir(parents=True)
            workspace.sources.mkdir(parents=True)
            workspace.state.parent.mkdir(parents=True)
            (workspace.downloads / "old").write_text("downloads", encoding="utf-8")
            (workspace.sources / "old").write_text("sources", encoding="utf-8")
            workspace.state.write_text("old-state", encoding="utf-8")
            transaction = workspace.staging / "transaction"
            staged_downloads = transaction / "downloads"
            staged_sources = transaction / "sources"
            staged_state = transaction / "state.json"
            staged_downloads.mkdir(parents=True)
            staged_sources.mkdir()
            (staged_downloads / "new").touch()
            (staged_sources / "new").touch()
            staged_state.write_text("new-state", encoding="utf-8")
            real_replace = os.replace

            def replace(source: Path, destination: Path) -> None:
                if Path(source) == staged_sources:
                    raise OSError("injected publication failure")
                real_replace(source, destination)

            with mock.patch("bbb_tools.vanilla.prepare.os.replace", side_effect=replace):
                with self.assertRaisesRegex(OSError, "injected publication failure"):
                    _publish(
                        workspace,
                        staged_downloads=staged_downloads,
                        staged_sources=staged_sources,
                        staged_state=staged_state,
                        transaction=transaction,
                    )
            self.assertEqual(
                (workspace.downloads / "old").read_text(encoding="utf-8"),
                "downloads",
            )
            self.assertEqual((workspace.sources / "old").read_text(encoding="utf-8"), "sources")
            self.assertEqual(workspace.state.read_text(encoding="utf-8"), "old-state")

    def test_failure_categories_have_distinct_stable_exit_codes(self) -> None:
        failures = (
            WorkspaceMissing,
            DownloadFailure,
            DigestMismatch,
            WorkspaceLocked,
            JavaIncompatible,
            DecompileFailure,
            IntegrityFailure,
            UnsafeClean,
        )
        self.assertEqual(len({failure.exit_code for failure in failures}), len(failures))

    def test_prepare_refuses_a_cache_inside_the_repository(self) -> None:
        workspace = VanillaWorkspace(
            Path(__file__).resolve().parents[2] / ".bbb-vanilla-test", "26.1"
        )
        with self.assertRaisesRegex(IntegrityFailure, "inside the bbb worktree"):
            _validate_cache_location(workspace)

    def test_committed_lock_is_complete_and_sorted(self) -> None:
        _, lock = load_lock("26.1")
        self.assertEqual(lock["java"]["major"], 25)
        self.assertEqual(len(lock["libraries"]), 107)
        paths = [value["path"] for value in lock["libraries"]]
        self.assertEqual(paths, sorted(paths))

    def test_only_download_module_imports_urllib(self) -> None:
        vanilla = Path(__file__).resolve().parents[1] / "bbb_tools" / "vanilla"
        importers = [
            path.name
            for path in vanilla.glob("*.py")
            if "urllib" in path.read_text(encoding="utf-8")
        ]
        self.assertEqual(importers, ["download.py"])
