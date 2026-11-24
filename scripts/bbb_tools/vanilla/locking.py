"""Per-version inter-process lock used by prepare and clean."""

from __future__ import annotations

import errno
import json
import os
import socket
import time
from pathlib import Path
from types import TracebackType
from typing import TextIO

from .errors import WorkspaceLocked

if os.name == "nt":
    import msvcrt

    # Lock beyond the JSON so a contending process can still read the owner.
    _LOCK_OFFSET = 1 << 30

    def _lock(file: TextIO, wait: bool) -> None:
        file.seek(_LOCK_OFFSET)
        while True:
            try:
                msvcrt.locking(file.fileno(), msvcrt.LK_NBLCK, 1)
                return
            except OSError as error:
                if error.errno != errno.EACCES:
                    raise
                if not wait:
                    raise BlockingIOError from error
                time.sleep(0.1)

    def _unlock(file: TextIO) -> None:
        file.seek(_LOCK_OFFSET)
        msvcrt.locking(file.fileno(), msvcrt.LK_UNLCK, 1)

else:
    import fcntl

    def _lock(file: TextIO, wait: bool) -> None:
        fcntl.flock(file.fileno(), fcntl.LOCK_EX | (0 if wait else fcntl.LOCK_NB))

    def _unlock(file: TextIO) -> None:
        fcntl.flock(file.fileno(), fcntl.LOCK_UN)


class VersionLock:
    def __init__(self, path: Path, *, wait: bool = False) -> None:
        self.path = path
        self.wait = wait
        self._file = None

    def __enter__(self) -> VersionLock:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._file = self.path.open("a+", encoding="utf-8")
        try:
            _lock(self._file, self.wait)
        except BlockingIOError as error:
            self._file.seek(0)
            owner = self._file.read().strip() or "unknown owner"
            self._file.close()
            self._file = None
            raise WorkspaceLocked(f"Vanilla workspace is locked by {owner}: {self.path}") from error
        owner = {
            "pid": os.getpid(),
            "host": socket.gethostname(),
            "started_unix": int(time.time()),
        }
        self._file.seek(0)
        self._file.truncate()
        json.dump(owner, self._file, sort_keys=True)
        self._file.flush()
        os.fsync(self._file.fileno())
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        if self._file is None:
            return
        self._file.seek(0)
        self._file.truncate()
        self._file.flush()
        _unlock(self._file)
        self._file.close()
        self._file = None
