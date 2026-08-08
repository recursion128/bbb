import argparse
import importlib.util
import json
import unittest
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "seat_inject.py"
SPEC = importlib.util.spec_from_file_location("seat_inject", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
seat_inject = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(seat_inject)


def driver_args(**overrides) -> argparse.Namespace:
    values = {
        "hold_seconds": seat_inject.DEFAULT_HOLD_SECONDS,
        "settle_seconds": 0.0,
        "output": "HEADLESS-1",
        "device_socket": "/tmp/does-not-exist.sock",
        "display": seat_inject.DEFAULT_DISPLAY,
        "window_class": seat_inject.DEFAULT_WINDOW_CLASS,
    }
    values.update(overrides)
    return argparse.Namespace(**values)


def tree(*windows: dict) -> str:
    return json.dumps({"type": "root", "nodes": list(windows)})


def window(
    identifier: int,
    pid: int,
    name: str,
    focused: bool = False,
    rect: dict | None = None,
) -> dict:
    result = {
        "type": "con",
        "id": identifier,
        "pid": pid,
        "name": name,
        "focused": focused,
    }
    if rect is not None:
        result["rect"] = rect
    return result


class SeatTargetTests(unittest.TestCase):
    """A gate run maps more than one client, and a wlroots compositor focuses
    whichever mapped last, so picking the newest window types into a client
    nobody is reading -- with no error to show for it."""

    def build(self, tree_json: str, argv_by_pid: dict[int, list[str]]):
        driver = seat_inject.WlrootsDriver(driver_args())
        run = mock.patch.object(seat_inject, "_run", return_value=tree_json)
        argv = mock.patch.object(
            seat_inject, "_process_arguments", lambda pid: argv_by_pid.get(pid, [])
        )
        return driver, run, argv

    def test_the_target_is_the_client_with_a_control_endpoint(self):
        driver, run, argv = self.build(
            tree(window(1, 10, "acceptance"), window(2, 20, "bbb-native", focused=True)),
            {10: ["bbb-native"], 20: ["bbb-native", "--control", "127.0.0.1:9999"]},
        )
        with run, argv:
            self.assertEqual(driver._target()["id"], 2)

    def test_a_later_mapped_window_does_not_steal_the_target(self):
        driver, run, argv = self.build(
            tree(window(2, 20, "bbb-native"), window(3, 30, "acceptance", focused=True)),
            {20: ["bbb-native", "--control", "127.0.0.1:9999"], 30: ["bbb-native"]},
        )
        with run, argv:
            self.assertEqual(driver._target()["id"], 2)

    def test_no_controlled_window_is_an_error_not_a_guess(self):
        driver, run, argv = self.build(
            tree(window(1, 10, "acceptance")), {10: ["bbb-native"]}
        )
        with run, argv, self.assertRaisesRegex(
            seat_inject.InjectError, "no mapped window belongs to a client"
        ):
            driver._target()

    def test_two_controlled_windows_are_ambiguous_rather_than_first_wins(self):
        driver, run, argv = self.build(
            tree(window(1, 10, "a"), window(2, 20, "b")),
            {10: ["bbb-native", "--control", "a"], 20: ["bbb-native", "--control", "b"]},
        )
        with run, argv, self.assertRaisesRegex(seat_inject.InjectError, "ambiguous"):
            driver._target()

    def test_an_empty_compositor_is_an_error(self):
        driver, run, argv = self.build(tree(), {})
        with run, argv, self.assertRaisesRegex(
            seat_inject.InjectError, "no toplevel window"
        ):
            driver._target()

    def test_window_receipt_includes_global_logical_origin(self):
        driver, run, argv = self.build(
            tree(
                window(
                    2,
                    20,
                    "bbb-native",
                    focused=True,
                    rect={"x": 37, "y": 19, "width": 1280, "height": 720},
                )
            ),
            {20: ["bbb-native", "--control", "127.0.0.1:9999"]},
        )
        with run, argv, mock.patch.object(driver, "_output_scale", return_value=1.0):
            receipt = driver.window()

        self.assertEqual(receipt["logical_x"], 37)
        self.assertEqual(receipt["logical_y"], 19)
        self.assertEqual(receipt["logical_width"], 1280)
        self.assertEqual(receipt["logical_height"], 720)


class SeatKeyRepeatTests(unittest.TestCase):
    """The gate holds keys deliberately; under a 600ms/25Hz repeat one hold
    becomes twenty-odd presses, which leaves a toggle such as the pause screen
    back where it started -- and reads as the key never arriving."""

    def inputs(self, *devices: dict) -> str:
        return json.dumps(list(devices))

    def test_a_repeating_keyboard_is_refused_by_name_without_configuration(self):
        driver = seat_inject.UinputDriver(driver_args())
        raw = self.inputs(
            {
                "identifier": "4660:22136:bbb-seat-virtual",
                "name": "bbb-seat-virtual",
                "type": "keyboard",
                "repeat_rate": 25,
            },
            {"identifier": "1:1:pointer", "type": "pointer"},
        )
        with mock.patch.object(seat_inject, "_run", return_value=raw):
            with self.assertRaisesRegex(seat_inject.InjectError, "repeat_rate 0"):
                driver._zero_key_repeat_receipt(configure=False)

    def test_a_silenced_keyboard_passes_with_a_receipt(self):
        driver = seat_inject.UinputDriver(driver_args())
        raw = self.inputs(
            {
                "identifier": "4660:22136:bbb-seat-virtual",
                "name": "bbb-seat-virtual",
                "type": "keyboard",
                "repeat_rate": 0,
            }
        )
        with mock.patch.object(seat_inject, "_run", return_value=raw):
            receipt = driver._zero_key_repeat_receipt(configure=False)

        self.assertEqual(receipt["keyboard_repeat_rate"], 0)
        self.assertEqual(
            receipt["keyboard_identifiers"], ["4660:22136:bbb-seat-virtual"]
        )

    def test_probe_configures_and_reads_back_zero_repeat(self):
        driver = seat_inject.UinputDriver(driver_args())
        repeated = self.inputs(
            {
                "identifier": "4660:22136:bbb-seat-virtual",
                "name": "bbb-seat-virtual",
                "type": "keyboard",
                "repeat_rate": 25,
            }
        )
        silenced = self.inputs(
            {
                "identifier": "4660:22136:bbb-seat-virtual",
                "name": "bbb-seat-virtual",
                "type": "keyboard",
                "repeat_rate": 0,
            }
        )
        calls = []
        responses = iter([repeated, '[{"success":true}]', silenced])

        def run(command):
            calls.append(command)
            return next(responses)

        with mock.patch.object(seat_inject, "_run", side_effect=run):
            receipt = driver._zero_key_repeat_receipt(configure=True)

        self.assertEqual(receipt["keyboard_repeat_rate"], 0)
        self.assertIn(
            [
                "swaymsg",
                "input",
                "4660:22136:bbb-seat-virtual",
                "repeat_rate",
                "0",
            ],
            calls,
        )


class SeatPointerProfileTests(unittest.TestCase):
    """Exact GUI targets require raw uinput deltas to remain logical pixels."""

    @staticmethod
    def inputs(profile: str) -> str:
        return json.dumps(
            [
                {
                    "identifier": "4660:22136:bbb-seat-virtual",
                    "name": "bbb-seat-virtual",
                    "type": "pointer",
                    "libinput": {"accel_profile": profile},
                }
            ]
        )

    def test_probe_configures_and_reads_back_a_flat_pointer_profile(self):
        driver = seat_inject.UinputDriver(driver_args())
        calls = []
        responses = iter(
            [
                self.inputs("adaptive"),
                '[{"success":true}]',
                self.inputs("flat"),
            ]
        )

        def run(command):
            calls.append(command)
            return next(responses)

        with mock.patch.object(seat_inject, "_run", side_effect=run):
            receipt = driver._flat_pointer_profile_receipt(configure=True)

        self.assertEqual(receipt["pointer_accel_profile"], "flat")
        self.assertEqual(
            receipt["pointer_identifiers"], ["4660:22136:bbb-seat-virtual"]
        )
        self.assertIn(
            [
                "swaymsg",
                "input",
                "4660:22136:bbb-seat-virtual",
                "accel_profile",
                "flat",
            ],
            calls,
        )

    def test_window_receipt_refuses_profile_drift(self):
        driver = seat_inject.UinputDriver(driver_args())
        with mock.patch.object(seat_inject, "_run", return_value=self.inputs("adaptive")):
            with self.assertRaisesRegex(seat_inject.InjectError, "needs flat acceleration"):
                driver._flat_pointer_profile_receipt(configure=False)

    def test_missing_gate_pointer_is_an_attributable_error(self):
        driver = seat_inject.UinputDriver(driver_args())
        with mock.patch.object(seat_inject, "_run", return_value="[]"):
            with self.assertRaisesRegex(seat_inject.InjectError, "no 'bbb-seat-virtual'"):
                driver._flat_pointer_profile_receipt(configure=True)


class SeatStaleDaemonTests(unittest.TestCase):
    """The seat device outlives edits to its own source, so a key added since
    it started is simply absent -- discovered, without this check, only when the
    run presses it tens of minutes in."""

    def test_a_daemon_missing_a_key_is_refused_by_name(self):
        driver = seat_inject.UinputDriver(driver_args())
        served = "ok " + " ".join(
            sorted((set(seat_inject.KEYSYMS) | set(seat_inject.TYPEABLE_LETTERS)) - {"fullscreen"})
        )
        with mock.patch.object(seat_inject.seat_device, "request", return_value=served):
            with self.assertRaisesRegex(seat_inject.InjectError, "fullscreen"):
                driver._require_current_key_table()

    def test_a_current_daemon_passes(self):
        driver = seat_inject.UinputDriver(driver_args())
        served = "ok " + " ".join(
            sorted(set(seat_inject.KEYSYMS) | set(seat_inject.TYPEABLE_LETTERS))
        )
        with mock.patch.object(seat_inject.seat_device, "request", return_value=served):
            driver._require_current_key_table()


class SeatVocabularyTests(unittest.TestCase):
    def test_a_daemon_that_cannot_type_is_refused(self):
        """Typing is how an input method gets something to compose, so a device
        without the letters is as stale as one missing a gate key."""

        driver = seat_inject.UinputDriver(driver_args())
        served = "ok " + " ".join(sorted(seat_inject.KEYSYMS))
        with mock.patch.object(seat_inject.seat_device, "request", return_value=served):
            with self.assertRaisesRegex(seat_inject.InjectError, "'n'"):
                driver._require_current_key_table()

    def test_every_gate_key_the_injector_knows_has_a_physical_code(self):
        """The two tables are written separately -- one in keysyms for the
        compositor drivers, one in evdev codes for the device -- so a name that
        exists in only one is a key that fails the moment it is pressed."""

        missing = sorted(set(seat_inject.KEYSYMS) - set(seat_inject.seat_device.KEY_CODES))

        self.assertEqual(missing, [])


class SeatDriverContractTests(unittest.TestCase):
    def test_the_default_driver_is_the_held_open_uinput_device(self):
        """The virtual-keyboard protocol carries keysyms on a caller-supplied
        keymap, so a client reading physical key codes sees whichever scancode
        the tool picked; uinput is the only driver that keeps that honest."""

        self.assertEqual(
            seat_inject._build_parser().parse_args(["probe"]).driver, "uinput"
        )
        self.assertIs(seat_inject.DRIVERS["uinput"], seat_inject.UinputDriver)

    def test_every_uinput_action_re_establishes_focus_first(self):
        """Any client that maps later takes focus away, and a client that loses
        focus in Play pauses itself, so focus is not a once-per-run step."""

        driver = seat_inject.UinputDriver(driver_args())
        for action, arguments in (
            ("key", ("escape",)),
            ("chord", ("ctrl", "paste")),
            ("move", (200, 0)),
            ("click", ("left",)),
        ):
            with self.subTest(action=action):
                with mock.patch.object(driver, "focus") as focus, mock.patch.object(
                    driver, "_send"
                ) as send:
                    getattr(driver, action)(*arguments)
                self.assertEqual(focus.call_count, 1)
                self.assertEqual(send.call_count, 1)

    def test_the_hold_outlasts_a_frame_and_stays_under_a_repeat_delay(self):
        """A twelve-millisecond tap lands entirely between two frames of a
        client uploading terrain; a hold past the usual 600ms repeat delay
        would double-fire on a compositor that was not silenced."""

        self.assertGreaterEqual(seat_inject.DEFAULT_HOLD_SECONDS, 0.2)
        self.assertLess(seat_inject.DEFAULT_HOLD_SECONDS, 0.6)


if __name__ == "__main__":
    unittest.main()
