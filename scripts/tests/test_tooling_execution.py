from __future__ import annotations

import os
import signal
import sys
import time
import unittest
from unittest.mock import patch

import pytest

from bbb_tools.host import capabilities as host_capabilities
from bbb_tools.execution.deadline import Deadline
from bbb_tools.execution.process import ManagedProcess, ProcessFailure
from bbb_tools.host.capabilities import (
    HostCapabilities,
    _vulkan_loader_available,
    missing_for_profile,
)
from bbb_tools.input.contract import InputBackend


class RecordingInput(InputBackend):
    def __init__(self) -> None:
        super().__init__()
        self.events: list[tuple[str, str, bool] | tuple[str, float, float]] = []

    def key_event(self, key: str, pressed: bool) -> None:
        self.events.append(("key", key, pressed))

    def button_event(self, button: str, pressed: bool) -> None:
        self.events.append(("button", button, pressed))

    def move(self, dx: float, dy: float) -> None:
        self.events.append(("move", dx, dy))


class ToolingExecutionTests(unittest.TestCase):
    def test_vulkan_loader_probe_uses_the_platform_linker_inventory(self) -> None:
        with patch.object(host_capabilities, "find_library", return_value="libvulkan.so.1"):
            self.assertTrue(_vulkan_loader_available())

    def test_deadline_uses_bounded_monotonic_remaining_time(self) -> None:
        deadline = Deadline.after(0.02)
        self.assertGreater(deadline.remaining(), 0)
        time.sleep(0.03)
        self.assertTrue(deadline.expired())

    @pytest.mark.integration
    def test_managed_process_timeout_kills_the_process_group(self) -> None:
        command = [
            sys.executable,
            "-c",
            "import subprocess,time; subprocess.Popen(['sleep','30']); time.sleep(30)",
        ]
        process = ManagedProcess(command).start()
        with self.assertRaises(ProcessFailure):
            process.communicate(0.05)
        with self.assertRaises(ProcessLookupError):
            os.killpg(process.pid, 0)

    def test_input_context_releases_pressed_state_after_failure(self) -> None:
        backend = RecordingInput()
        backend._pressed_keys["w"] = None
        backend._pressed_buttons["left"] = None
        backend.release_all()
        self.assertIn(("key", "w", False), backend.events)
        self.assertIn(("button", "left", False), backend.events)

    def test_input_context_releases_nested_inputs_in_reverse_order(self) -> None:
        backend = RecordingInput()
        backend._pressed_keys["shift"] = None
        backend._pressed_keys["w"] = None
        backend._pressed_buttons["left"] = None
        backend.release_all()
        self.assertEqual(
            backend.events,
            [
                ("key", "w", False),
                ("key", "shift", False),
                ("button", "left", False),
            ],
        )

    def test_profiles_are_policy_over_host_facts(self) -> None:
        capabilities = HostCapabilities(
            os="darwin",
            architecture="arm64",
            python="3.12",
            python_312=True,
            cargo=True,
            rustc=True,
            git=True,
            java=False,
            graphical_session=True,
            wayland=False,
            vulkan_loader=False,
            linux_uinput=False,
            linux_seat_tools=False,
            libei=False,
            macos_quartz=True,
            macos_accessibility_trusted=True,
        )
        self.assertEqual(missing_for_profile(capabilities, "mac-dev"), [])
        self.assertEqual(missing_for_profile(capabilities, "mac-input"), [])
        self.assertIn("linux_uinput", missing_for_profile(capabilities, "linux-release"))


if __name__ == "__main__":
    unittest.main()
