from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"
sys.path.insert(0, str(SCRIPTS))

from file_drop_capability import (  # noqa: E402
    FileDropCapabilityError,
    OWNER_IDS,
    analyze_public_event,
    analyze_x11,
    render_report,
    resolve_winit_root,
)


class FileDropCapabilityTests(unittest.TestCase):
    def test_winit_metadata_is_filtered_to_the_rust_host(self) -> None:
        host = "aarch64-apple-darwin"
        manifest = "/tmp/winit-0.29.15/Cargo.toml"
        completed = (
            subprocess.CompletedProcess(
                ["rustc", "-vV"], 0, stdout=f"rustc fixture\nhost: {host}\n", stderr=""
            ),
            subprocess.CompletedProcess(
                ["cargo", "metadata"],
                0,
                stdout=json.dumps(
                    {
                        "packages": [
                            {
                                "name": "winit",
                                "version": "0.29.15",
                                "manifest_path": manifest,
                            }
                        ]
                    }
                ),
                stderr="",
            ),
        )
        with mock.patch(
            "file_drop_capability.subprocess.run", side_effect=completed
        ) as run:
            self.assertEqual(
                resolve_winit_root(Path("/tmp/repo")), Path(manifest).resolve().parent
            )

        metadata_command = run.call_args_list[1].args[0]
        self.assertEqual(metadata_command[-2:], ["--filter-platform", host])

    def test_checked_receipt_matches_verified_vanilla_and_pinned_winit(self) -> None:
        source_root = Path(
            subprocess.run(
                [str(SCRIPTS / "vanilla"), "path", "26.1"],
                cwd=REPO_ROOT,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
        )
        workspace_root = Path(
            subprocess.run(
                [str(SCRIPTS / "vanilla"), "path", "26.1", "--kind", "root"],
                cwd=REPO_ROOT,
                check=True,
                text=True,
                stdout=subprocess.PIPE,
            ).stdout.strip()
        )
        expected = (
            REPO_ROOT / "docs/evidence/vanilla-26.1/file-drop-capability.json"
        ).read_text(encoding="utf-8")
        self.assertEqual(
            render_report(
                REPO_ROOT,
                source_root,
                workspace_root / "downloads" / "26.1",
                resolve_winit_root(REPO_ROOT),
            ),
            expected,
        )
        receipt = json.loads(expected)
        self.assertEqual(receipt["owner_ids"], list(OWNER_IDS))
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["status"], "scope-ruled-not-needed")
        self.assertEqual(
            receipt["scope_ruling"]["classification"],
            "not-needed-declared-linux-wayland",
        )
        self.assertEqual(
            receipt["vanilla_contract"]["official_runtime"]["java_runtime"],
            {"component": "java-runtime-epsilon", "majorVersion": 25},
        )
        self.assertTrue(
            receipt["vanilla_contract"]["official_runtime"]["lwjgl_core"][
                "multi_release"
            ]
        )
        selected = {
            item["path"]
            for item in receipt["vanilla_contract"]["official_runtime"][
                "lwjgl_core"
            ]["selected_java_25_entries"]
        }
        self.assertEqual(
            selected,
            {
                "META-INF/versions/11/org/lwjgl/system/MultiReleaseTextDecoding.class",
                "META-INF/versions/25/org/lwjgl/system/MemoryUtil.class",
            },
        )
        self.assertFalse(
            receipt["bbb_event_surface"]["adapter"]["failed_path_count_consumed"]
        )
        self.assertEqual(
            [
                item["path"]
                for item in receipt["vanilla_contract"]["on_files_drop_occurrences"]
            ],
            [
                "net/minecraft/client/MouseHandler.java",
                "net/minecraft/client/gui/screens/Screen.java",
                "net/minecraft/client/gui/screens/packs/PackSelectionScreen.java",
            ],
        )

    def test_public_failed_drop_event_forces_scope_reaudit(self) -> None:
        source = """\
pub enum WindowEvent {
    /// A file has been dropped into the window.
    ///
    /// When the user drops multiple files at once, this event will be emitted for each file
    /// separately.
    DroppedFile(PathBuf),
}
"""
        self.assertFalse(analyze_public_event(source)["failed_path_count_exposed"])
        changed = source.replace(
            "    DroppedFile(PathBuf),",
            "    DroppedFile(PathBuf),\n    DroppedFileFailed { count: usize },",
        )
        with self.assertRaisesRegex(
            FileDropCapabilityError, "failed-drop channel"
        ):
            analyze_public_event(changed)

    def test_x11_error_dispatch_forces_scope_reaudit(self) -> None:
        dnd = """\
pub enum DndDataParseError {
    EmptyData,
    InvalidUtf8(Utf8Error),
    HostnameSpecified(String),
    UnexpectedProtocol(String),
    UnresolvablePath(io::Error),
}
pub result: Option<Result<Vec<PathBuf>, DndDataParseError>>,
"""
        event = """\
if xev.message_type == atoms[XdndDrop] {
    if let Some(Ok(ref path_list)) = self.dnd.result {
        event: WindowEvent::DroppedFile(path.clone()),
    }
}
if xev.message_type == atoms[XdndLeave] {}
self.dnd.result = Some(parse_result);
"""
        self.assertEqual(analyze_x11(event, dnd)["parse_error_delivery"], "no-WindowEvent")
        changed = event.replace(
            "    if let Some(Ok(ref path_list)) = self.dnd.result {",
            "    if let Some(Err(error)) = self.dnd.result { send_failed(error); }\n"
            "    if let Some(Ok(ref path_list)) = self.dnd.result {",
        )
        with self.assertRaisesRegex(
            FileDropCapabilityError, "failed-drop channel"
        ):
            analyze_x11(changed, dnd)


if __name__ == "__main__":
    unittest.main()
