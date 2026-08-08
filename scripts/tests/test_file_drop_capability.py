from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import unittest


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
        expected = (
            REPO_ROOT / "docs/evidence/vanilla-26.1/file-drop-capability.json"
        ).read_text(encoding="utf-8")
        self.assertEqual(
            render_report(REPO_ROOT, source_root, resolve_winit_root(REPO_ROOT)),
            expected,
        )
        receipt = json.loads(expected)
        self.assertEqual(receipt["owner_ids"], list(OWNER_IDS))
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["status"], "blocked-external")

    def test_public_failed_drop_event_forces_blocker_reaudit(self) -> None:
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

    def test_x11_error_dispatch_forces_blocker_reaudit(self) -> None:
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
