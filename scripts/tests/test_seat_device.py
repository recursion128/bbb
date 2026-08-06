import importlib.util
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = REPO_ROOT / "scripts" / "seat_device.py"
SPEC = importlib.util.spec_from_file_location("seat_device", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
seat_device = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(seat_device)


class RecordingDevice:
    """Stands in for the uinput device, recording what would be emitted."""

    def __init__(self):
        self.calls = []

    def key(self, name, hold_seconds):
        self.calls.append(("key", name, hold_seconds))

    def chord(self, modifier, name, hold_seconds):
        self.calls.append(("chord", modifier, name, hold_seconds))

    def click(self, button, hold_seconds):
        self.calls.append(("click", button, hold_seconds))

    def move(self, dx, dy):
        self.calls.append(("move", dx, dy))


class SeatKeyTableTests(unittest.TestCase):
    def test_keys_are_evdev_positions_not_keysyms(self):
        """The point of going through uinput is that the client reads physical
        key codes; a table built from keysyms would inject the wrong keys while
        still looking like it worked."""

        # Linux input-event-codes.h.
        for name, code in (
            ("escape", 1),
            ("forward", 17),
            ("back", 31),
            ("left", 30),
            ("right", 32),
            ("inventory", 18),
            ("chat", 20),
            ("paste", 47),
            ("ctrl", 29),
            ("sneak", 42),
            ("jump", 57),
        ):
            with self.subTest(key=name):
                self.assertEqual(seat_device.KEY_CODES[name], code)

    def test_hotbar_keys_cover_the_nine_slots_in_row_order(self):
        self.assertEqual(
            [seat_device.KEY_CODES[f"hotbar{slot}"] for slot in range(1, 10)],
            list(range(2, 11)),  # KEY_1..KEY_9
        )
        self.assertNotIn("hotbar10", seat_device.KEY_CODES)

    def test_default_hold_outlasts_a_frame(self):
        """A twelve-millisecond tap lands entirely between two frames of a
        client uploading terrain, which reads as the seat not working."""

        self.assertGreaterEqual(seat_device.DEFAULT_HOLD_SECONDS, 1.0)


class SeatCommandTests(unittest.TestCase):
    def setUp(self):
        self.device = RecordingDevice()

    def test_commands_reach_the_device_with_the_default_hold(self):
        for line, expected in (
            ("key escape", ("key", "escape", seat_device.DEFAULT_HOLD_SECONDS)),
            ("key escape 0.25", ("key", "escape", 0.25)),
            (
                "chord ctrl paste",
                ("chord", "ctrl", "paste", seat_device.DEFAULT_HOLD_SECONDS),
            ),
            ("click left", ("click", "left", seat_device.DEFAULT_HOLD_SECONDS)),
            ("move 200 0", ("move", 200, 0)),
        ):
            with self.subTest(line=line):
                self.device.calls.clear()
                self.assertEqual(seat_device._handle(self.device, line), "ok")
                self.assertEqual(self.device.calls, [expected])

    def test_ping_answers_without_touching_the_device(self):
        self.assertEqual(seat_device._handle(self.device, "ping"), "ok")
        self.assertEqual(self.device.calls, [])

    def test_the_daemon_reports_the_key_table_it_was_started_with(self):
        """It is deliberately long-lived, so it keeps running the code it
        started with; a caller has to be able to see that table to refuse a
        daemon that predates a key it needs."""

        answer = seat_device._handle(self.device, "keys")
        typeable = {name for name in seat_device.LETTER_CODES if not name.isspace()}

        self.assertTrue(answer.startswith("ok "))
        self.assertEqual(set(answer.split()[1:]), set(seat_device.KEY_CODES) | typeable)
        # A bare space cannot survive a whitespace-separated answer, so it is
        # reported under the gate key that already owns it.
        self.assertIn(" ", seat_device.LETTER_CODES)
        self.assertNotIn(" ", answer.split()[1:])
        self.assertEqual(self.device.calls, [])

    def test_typed_text_keeps_its_spaces(self):
        """The command's argument is the rest of the line, because a typed
        space is a key like any other."""

        typed = []
        self.device.text = lambda characters: typed.append(characters)

        self.assertEqual(seat_device._handle(self.device, "text ni hao"), "ok")

        self.assertEqual(typed, ["ni hao"])
        self.assertTrue(seat_device._handle(self.device, "text").startswith("error"))

    def test_malformed_requests_answer_error_and_inject_nothing(self):
        for line in ("", "key", "move 200", "move x 0", "wiggle"):
            with self.subTest(line=line):
                self.assertTrue(seat_device._handle(self.device, line).startswith("error"))
        self.assertEqual(self.device.calls, [])


class SeatTypingTests(unittest.TestCase):
    def test_typing_presses_one_physical_key_per_character(self):
        """An input method turns these into something else entirely, which is
        the point: the receipt is that the client did not get what was typed."""

        emitted = []
        device = seat_device.VirtualSeatDevice.__new__(seat_device.VirtualSeatDevice)
        device._emit = lambda event_type, code, value: emitted.append((code, value))
        device._sync = lambda: None
        seat_device.VirtualSeatDevice.text(device, "hi")

        self.assertEqual(
            emitted,
            [
                (seat_device.LETTER_CODES["h"], 1),
                (seat_device.LETTER_CODES["h"], 0),
                (seat_device.LETTER_CODES["i"], 1),
                (seat_device.LETTER_CODES["i"], 0),
            ],
        )

    def test_an_unmapped_character_is_refused_rather_than_skipped(self):
        device = seat_device.VirtualSeatDevice.__new__(seat_device.VirtualSeatDevice)
        device._emit = lambda *_: None
        device._sync = lambda: None

        with self.assertRaisesRegex(seat_device.SeatDeviceError, "no physical key"):
            seat_device.VirtualSeatDevice.text(device, "ni3")


class SeatMotionTests(unittest.TestCase):
    def test_motion_is_split_into_steps_that_sum_to_the_request(self):
        """One jump can be sampled as a single teleport -- or missed entirely
        by a client that reads the delta once per frame -- so the device moves
        the way a real pointer does."""

        emitted = []
        device = seat_device.VirtualSeatDevice.__new__(seat_device.VirtualSeatDevice)
        device._emit = lambda event_type, code, value: emitted.append(
            (event_type, code, value)
        )
        device._sync = lambda: None
        seat_device.VirtualSeatDevice.move(device, 200, -90, steps=10)

        self.assertGreaterEqual(len(emitted), 10)
        self.assertEqual(
            sum(value for _, code, value in emitted if code == seat_device.REL_X), 200
        )
        self.assertEqual(
            sum(value for _, code, value in emitted if code == seat_device.REL_Y), -90
        )
        self.assertTrue(
            all(
                event_type == seat_device.EV_REL and abs(value) < 200
                for event_type, _, value in emitted
            )
        )


if __name__ == "__main__":
    unittest.main()
