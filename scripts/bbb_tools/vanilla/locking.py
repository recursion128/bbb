"""Per-version inter-process lock used by prepare and clean."""

from __future__ import annotations

import fcntl
import json
import os
import socket
import time
from pathlib import Path
from types import TracebackType

from .errors import WorkspaceLocked


class VersionLock:
    def __init__(self, path: Path, *, wait: bool = False) -> None:
        self.path = path
        self.wait = wait
        self._file = None

    def __enter__(self) -> VersionLock:
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._file = self.path.open("a+", encoding="utf-8")
        flags = fcntl.LOCK_EX | (0 if self.wait else fcntl.LOCK_NB)
        try:
            fcntl.flock(self._file.fileno(), flags)
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
        fcntl.flock(self._file.fileno(), fcntl.LOCK_UN)
        self._file.close()
        self._file = None
