"""Backend-neutral input actions and fail-safe release tracking."""

from __future__ import annotations

import time
from abc import ABC, abstractmethod
from types import TracebackType


class InputError(RuntimeError):
    pass


class InputBackend(ABC):
    def __init__(self) -> None:
        # Dict insertion order is the press order. Releases deliberately walk
        # it backwards so nested chords unwind like a stack.
        self._pressed_keys: dict[str, None] = {}
        self._pressed_buttons: dict[str, None] = {}

    @abstractmethod
    def key_event(self, key: str, pressed: bool) -> None: ...

    @abstractmethod
    def button_event(self, button: str, pressed: bool) -> None: ...

    @abstractmethod
    def move(self, dx: float, dy: float) -> None: ...

    @staticmethod
    def _hold(hold_seconds: float) -> None:
        if hold_seconds < 0:
            raise InputError("input hold duration must not be negative")
        if hold_seconds:
            time.sleep(hold_seconds)

    def key(self, key: str, *, hold_seconds: float = 0.0) -> None:
        self._pressed_keys[key] = None
        self.key_event(key, True)
        self._hold(hold_seconds)
        self.key_event(key, False)
        self._pressed_keys.pop(key, None)

    def chord(self, modifier: str, key: str, *, hold_seconds: float = 0.0) -> None:
        self._pressed_keys[modifier] = None
        self.key_event(modifier, True)
        self._pressed_keys[key] = None
        self.key_event(key, True)
        self._hold(hold_seconds)
        self.key_event(key, False)
        self._pressed_keys.pop(key, None)
        self.key_event(modifier, False)
        self._pressed_keys.pop(modifier, None)

    def click(self, button: str, *, hold_seconds: float = 0.0) -> None:
        self._pressed_buttons[button] = None
        self.button_event(button, True)
        self._hold(hold_seconds)
        self.button_event(button, False)
        self._pressed_buttons.pop(button, None)

    def release_all(self) -> None:
        failures: list[str] = []
        for key in reversed(tuple(self._pressed_keys)):
            try:
                self.key_event(key, False)
            except Exception as error:  # keep trying to release every held input
                failures.append(f"key {key!r}: {error}")
            else:
                self._pressed_keys.pop(key, None)
        for button in reversed(tuple(self._pressed_buttons)):
            try:
                self.button_event(button, False)
            except Exception as error:  # keep trying to release every held input
                failures.append(f"button {button!r}: {error}")
            else:
                self._pressed_buttons.pop(button, None)
        if failures:
            raise InputError("could not release held input: " + "; ".join(failures))

    def __enter__(self) -> InputBackend:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> bool | None:
        try:
            self.release_all()
        except InputError as release_error:
            if exc is None:
                raise
            exc.add_note(str(release_error))
        return None
